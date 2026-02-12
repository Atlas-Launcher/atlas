import { computed, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getCurrent, onOpenUrl } from "@tauri-apps/plugin-deep-link";
import { openUrl } from "@tauri-apps/plugin-opener";
import type {
  AtlasProfile,
  AuthFlow,
  DeviceCodeResponse,
  LauncherLinkComplete,
  LauncherLinkSession,
  Profile
} from "@/types/auth";

interface AuthDeps {
  setStatus: (message: string) => void;
  pushLog: (entry: string) => void;
  run: <T>(task: () => Promise<T>) => Promise<T | undefined>;
  onUnhandledDeepLink?: (url: string) => void;
}

function resolveAuthFlow(value: string): AuthFlow {
  return value === "device_code" ? "device_code" : "deeplink";
}

function resolveDeepLinkTarget(url: string): "microsoft" | "atlas" | null {
  try {
    const parsed = new URL(url);
    const target = `${parsed.hostname}${parsed.pathname}`.toLowerCase();
    if (target.includes("signin")) {
      return "atlas";
    }
    if (target.includes("auth")) {
      return "microsoft";
    }
  } catch {
    return null;
  }
  return null;
}

function atlasIdentity(profile: AtlasProfile): string {
  return profile.name?.trim() || profile.email?.trim() || profile.id;
}

export function useAuth({ setStatus, pushLog, run, onUnhandledDeepLink }: AuthDeps) {
  const authFlow = resolveAuthFlow((import.meta.env.VITE_AUTH_FLOW ?? "deeplink").toLowerCase());
  const deviceCode = ref<DeviceCodeResponse | null>(null);
  const pendingDeeplink = ref<string | null>(null);
  const profile = ref<Profile | null>(null);
  const atlasProfile = ref<AtlasProfile | null>(null);
  const atlasPendingDeeplink = ref<string | null>(null);
  const launcherLinkSession = ref<LauncherLinkSession | null>(null);
  const authInFlight = ref(false);
  const atlasAuthInFlight = ref(false);
  let deviceLoginAttempt = 0;
  let launcherLinkPollAttempt = 0;
  let launcherLinkPollTimer: number | undefined;
  const LAUNCHER_LINK_STORAGE_KEY = "atlas.launcherLinkSession";
  const LAUNCHER_LINK_POLL_INTERVAL_MS = 3000;

  const isSigningIn = computed(
    () =>
      authInFlight.value ||
      atlasAuthInFlight.value ||
      !!deviceCode.value ||
      !!pendingDeeplink.value ||
      !!atlasPendingDeeplink.value
  );

  function readStoredLauncherLinkSession(): LauncherLinkSession | null {
    if (typeof window === "undefined") {
      return null;
    }
    try {
      const raw = window.localStorage.getItem(LAUNCHER_LINK_STORAGE_KEY);
      if (!raw) {
        return null;
      }
      const parsed = JSON.parse(raw) as LauncherLinkSession;
      if (!parsed?.linkSessionId || !parsed?.expiresAt) {
        return null;
      }
      if (Date.parse(parsed.expiresAt) <= Date.now()) {
        window.localStorage.removeItem(LAUNCHER_LINK_STORAGE_KEY);
        return null;
      }
      return parsed;
    } catch {
      return null;
    }
  }

  function writeStoredLauncherLinkSession(session: LauncherLinkSession | null) {
    if (typeof window === "undefined") {
      return;
    }
    try {
      if (!session) {
        window.localStorage.removeItem(LAUNCHER_LINK_STORAGE_KEY);
        return;
      }
      window.localStorage.setItem(LAUNCHER_LINK_STORAGE_KEY, JSON.stringify(session));
    } catch {
      // Ignore storage errors; session remains in-memory.
    }
  }

  launcherLinkSession.value = readStoredLauncherLinkSession();

  function clearLauncherLinkPoll() {
    if (launcherLinkPollTimer) {
      window.clearTimeout(launcherLinkPollTimer);
      launcherLinkPollTimer = undefined;
    }
  }

  function isLauncherLinkExpired(session: LauncherLinkSession) {
    const expiresAt = Date.parse(session.expiresAt);
    return Number.isFinite(expiresAt) && expiresAt <= Date.now();
  }

  async function tryCompleteLauncherLink(session: LauncherLinkSession) {
    if (!profile.value) {
      return {
        success: false,
        retryable: false,
        message: "Missing Minecraft profile."
      };
    }
    try {
      const result = await invoke<LauncherLinkComplete>("complete_launcher_link_session", {
        linkSessionId: session.linkSessionId,
        proof: session.proof,
        minecraftUuid: profile.value.id,
        minecraftName: profile.value.name
      });

      if (result.warning) {
        pushLog(`Launcher link warning: ${result.warning}`);
      }

      if (result.success) {
        launcherLinkSession.value = null;
        writeStoredLauncherLinkSession(null);
        await restoreAtlasSession();
        setStatus("Launcher link completed.");
        return {
          success: true,
          retryable: false,
          message: ""
        };
      }

      return {
        success: false,
        retryable: false,
        message: "Launcher link failed."
      };
    } catch (err) {
      const message = String(err);
      const retryable = /link session not claimed/i.test(message);
      return {
        success: false,
        retryable,
        message
      };
    }
  }

  function startLauncherLinkCompletionPoll(session: LauncherLinkSession) {
    if (!session) {
      return;
    }
    if (isLauncherLinkExpired(session)) {
      setStatus("Launcher link expired. Request a new link code.");
      launcherLinkSession.value = null;
      writeStoredLauncherLinkSession(null);
      return;
    }
    const attempt = ++launcherLinkPollAttempt;
    clearLauncherLinkPoll();

    const poll = async () => {
      if (attempt !== launcherLinkPollAttempt) {
        return;
      }
      if (isLauncherLinkExpired(session)) {
        setStatus("Launcher link expired. Request a new link code.");
        launcherLinkSession.value = null;
        writeStoredLauncherLinkSession(null);
        return;
      }
      const outcome = await tryCompleteLauncherLink(session);
      if (outcome.success) {
        clearLauncherLinkPoll();
        return;
      }
      if (!outcome.retryable) {
        setStatus(`Launcher link failed: ${outcome.message}`);
        clearLauncherLinkPoll();
        return;
      }
      launcherLinkPollTimer = window.setTimeout(poll, LAUNCHER_LINK_POLL_INTERVAL_MS);
    };

    launcherLinkPollTimer = window.setTimeout(poll, LAUNCHER_LINK_POLL_INTERVAL_MS);
  }

  async function waitForDeviceApproval(deviceCodeValue: string, attempt: number) {
    try {
      const result = await invoke<Profile>("complete_device_code", {
        deviceCode: deviceCodeValue
      });
      if (attempt !== deviceLoginAttempt) {
        return;
      }
      profile.value = result;
      deviceCode.value = null;
      authInFlight.value = false;
      setStatus(`Signed in as ${result.name}.`);
      // Auto-attempt to complete a stored launcher link session (if present)
      if (launcherLinkSession.value) {
        // fire-and-forget; completeLauncherLink internally handles retries/polling
        void run(async () => {
          try {
            setStatus("Completing launcher link...");
            const ok = await completeLauncherLink();
            if (ok) {
              pushLog("Launcher link auto-completed after Microsoft sign-in.");
            }
          } finally {
            // restore signed-in status message
            setStatus(`Signed in as ${result.name}.`);
          }
        });
      }
    } catch (err) {
      if (attempt !== deviceLoginAttempt) {
        return;
      }
      deviceCode.value = null;
      authInFlight.value = false;
      setStatus(`Login failed: ${String(err)}`);
    }
  }

  async function restoreSession() {
    try {
      const restored = await invoke<Profile | null>("restore_session");
      if (restored) {
        profile.value = restored;
        setStatus(`Signed in as ${restored.name}.`);
      }
    } catch (err) {
      pushLog(`Failed to restore session: ${String(err)}`);
    }
  }

  async function restoreAtlasSession() {
    try {
      const restored = await invoke<AtlasProfile | null>("restore_atlas_session");
      if (restored) {
        atlasProfile.value = restored;
        setStatus(`Signed in to Atlas Hub as ${atlasIdentity(restored)}.`);
      }
    } catch (err) {
      pushLog(`Failed to restore Atlas session: ${String(err)}`);
    }
  }

  async function restoreSessions() {
    await restoreSession();
    await restoreAtlasSession();
  }

  async function currentDeepLinkFor(
    target: "microsoft" | "atlas"
  ): Promise<string | null> {
    try {
      const current = await getCurrent();
      if (!current || current.length === 0) {
        return null;
      }
      return current.find((entry) => resolveDeepLinkTarget(entry) === target) ?? null;
    } catch (err) {
      pushLog(`Failed to read auth redirect: ${String(err)}`);
      return null;
    }
  }

  function handleDeepLink(url: string) {
    const target = resolveDeepLinkTarget(url);
    if (target === "atlas") {
      atlasPendingDeeplink.value = url;
      pushLog("Atlas auth redirect received.");
      void finishAtlasLogin(url);
      return;
    }
    if (target === "microsoft" && authFlow !== "device_code") {
      pendingDeeplink.value = url;
      pushLog("Microsoft auth redirect received.");
      void finishDeeplinkLogin(url);
      return;
    }
    onUnhandledDeepLink?.(url);
  }

  async function initDeepLink() {
    try {
      const current = await getCurrent();
      if (current?.length) {
        for (const url of current) {
          handleDeepLink(url);
        }
      }
    } catch (err) {
      pushLog(`Failed to read deep-link state: ${String(err)}`);
    }

    await onOpenUrl((urls) => {
      if (!urls || urls.length === 0) {
        return;
      }
      for (const url of urls) {
        handleDeepLink(url);
      }
    });
  }

  async function startLogin() {
    authInFlight.value = true;
    if (authFlow === "device_code") {
      await startDeviceLogin();
    } else {
      await startDeeplinkLogin();
    }
  }

  async function startDeviceLogin() {
    const attempt = ++deviceLoginAttempt;
    const response = await run(async () => {
      try {
        deviceCode.value = null;
        const nextDeviceCode = await invoke<DeviceCodeResponse>("start_device_code");
        deviceCode.value = nextDeviceCode;
        const url = nextDeviceCode.verification_uri_complete ?? nextDeviceCode.verification_uri;
        await openUrl(url);
        setStatus("Waiting for Microsoft sign-in approval in your browser.");
        return nextDeviceCode;
      } catch (err) {
        setStatus(`Login start failed: ${String(err)}`);
        authInFlight.value = false;
        return null;
      }
    });
    if (!response) {
      return;
    }
    void waitForDeviceApproval(response.device_code, attempt);
  }

  async function startDeeplinkLogin() {
    await run(async () => {
      try {
        pendingDeeplink.value = null;
        const authUrl = await invoke<string>("begin_deeplink_login");
        await openUrl(authUrl);
        setStatus("Finish signing in in your browser.");
      } catch (err) {
        setStatus(`Login start failed: ${String(err)}`);
        authInFlight.value = false;
      }
    });
  }

  async function finishDeeplinkLogin(callbackUrl?: string) {
    let url = callbackUrl ?? pendingDeeplink.value;
    if (!url) {
      url = await currentDeepLinkFor("microsoft");
    }
    if (!url) {
      setStatus("Missing auth redirect URL. Open the atlas://auth link to continue.");
      return;
    }

    await run(async () => {
      try {
        const result = await invoke<Profile>("complete_deeplink_login", {
          callbackUrl: url
        });
        profile.value = result;
        setStatus(`Signed in as ${result.name}.`);
        pendingDeeplink.value = null;
        authInFlight.value = false;
        // Auto-attempt launcher link completion if we have a stored session
        if (launcherLinkSession.value) {
          void run(async () => {
            try {
              setStatus("Completing launcher link...");
              const ok = await completeLauncherLink();
              if (ok) {
                pushLog("Launcher link auto-completed after Microsoft sign-in.");
              }
            } finally {
              setStatus(`Signed in as ${result.name}.`);
            }
          });
        }
      } catch (err) {
        setStatus(`Login failed: ${String(err)}`);
        authInFlight.value = false;
      }
    });
  }

  async function startAtlasLogin() {
    atlasAuthInFlight.value = true;
    await run(async () => {
      try {
        atlasPendingDeeplink.value = null;
        const authUrl = await invoke<string>("begin_atlas_login");
        await openUrl(authUrl);
        setStatus("Finish Atlas sign-in in your browser.");
      } catch (err) {
        setStatus(`Atlas login start failed: ${String(err)}`);
        atlasAuthInFlight.value = false;
      }
    });
  }

  async function finishAtlasLogin(callbackUrl?: string) {
    let url = callbackUrl ?? atlasPendingDeeplink.value;
    if (!url) {
      url = await currentDeepLinkFor("atlas");
    }
    if (!url) {
      setStatus("Missing Atlas callback URL. Open the atlas://signin link to continue.");
      return;
    }

    await run(async () => {
      try {
        const result = await invoke<AtlasProfile>("complete_atlas_login", {
          callbackUrl: url
        });
        atlasProfile.value = result;
        setStatus(`Signed in to Atlas Hub as ${atlasIdentity(result)}.`);
        atlasPendingDeeplink.value = null;
        atlasAuthInFlight.value = false;
      } catch (err) {
        setStatus(`Atlas login failed: ${String(err)}`);
        atlasAuthInFlight.value = false;
      }
    });
  }

  async function signOut() {
    deviceLoginAttempt += 1;
    await run(async () => {
      try {
        await invoke("sign_out");
        profile.value = null;
        deviceCode.value = null;
        pendingDeeplink.value = null;
        launcherLinkSession.value = null;
        writeStoredLauncherLinkSession(null);
        setStatus("Signed out.");
      } catch (err) {
        setStatus(`Sign out failed: ${String(err)}`);
      }
    });
  }

  async function signOutAtlas() {
    await run(async () => {
      try {
        await invoke("atlas_sign_out");
        atlasProfile.value = null;
        atlasPendingDeeplink.value = null;
        launcherLinkSession.value = null;
        writeStoredLauncherLinkSession(null);
        setStatus("Signed out of Atlas Hub.");
      } catch (err) {
        setStatus(`Atlas sign out failed: ${String(err)}`);
      }
    });
  }

  async function createLauncherLink() {
    const result = await run(async () => {
      try {
        const session = await invoke<LauncherLinkSession>("create_launcher_link_session");
        launcherLinkSession.value = session;
        writeStoredLauncherLinkSession(session);
        setStatus("Launcher link code generated.");
        pushLog("Launcher link session created.");
        return session;
      } catch (err) {
        setStatus(`Launcher link failed: ${String(err)}`);
        pushLog(`Launcher link failed: ${String(err)}`);
        return null;
      }
    });
    return result ?? null;
  }

  async function completeLauncherLink() {
    if (!launcherLinkSession.value || !profile.value) {
      setStatus("Missing launcher link session or Minecraft profile.");
      pushLog("Launcher link completion blocked: missing session or profile.");
      return false;
    }

    const session = launcherLinkSession.value;
    const result = await run(async () => {
      try {
        const outcome = await tryCompleteLauncherLink(session);
        if (outcome.success) {
          return true;
        }
        if (outcome.retryable) {
          startLauncherLinkCompletionPoll(session);
          return false;
        }
        setStatus(`Launcher link failed: ${outcome.message}`);
        pushLog(`Launcher link failed: ${outcome.message}`);
        return false;
      } catch (err) {
        setStatus(`Launcher link failed: ${String(err)}`);
        pushLog(`Launcher link failed: ${String(err)}`);
        return false;
      }
    });

    return result === true;
  }

  return {
    authFlow,
    isSigningIn,
    profile,
    atlasProfile,
    launcherLinkSession,
    restoreSession,
    restoreAtlasSession,
    restoreSessions,
    initDeepLink,
    startLogin,
    startAtlasLogin,
    finishDeeplinkLogin,
    finishAtlasLogin,
    signOut,
    signOutAtlas,
    createLauncherLink,
    completeLauncherLink,
    startLauncherLinkCompletionPoll
  };
}
