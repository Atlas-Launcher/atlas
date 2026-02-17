import { computed, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getCurrent, onOpenUrl } from "@tauri-apps/plugin-deep-link";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { openUrl } from "@tauri-apps/plugin-opener";
import type {
  AtlasProfile,
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

function atlasIdentity(profile: AtlasProfile): string {
  return profile.name?.trim() || profile.email?.trim() || profile.id;
}

function resolveVerificationUrl(device: DeviceCodeResponse): string {
  if (device.verification_uri_complete?.trim()) {
    return device.verification_uri_complete;
  }

  const base = device.verification_uri?.trim();
  if (!base) {
    return "";
  }

  if (!device.user_code?.trim()) {
    return base;
  }

  try {
    const url = new URL(base);
    if (!url.searchParams.has("user_code")) {
      url.searchParams.set("user_code", device.user_code);
    }
    return url.toString();
  } catch {
    return base;
  }
}

function hasVerificationUriComplete(device: DeviceCodeResponse): boolean {
  return !!device.verification_uri_complete?.trim();
}

export function useAuth({ setStatus, pushLog, run, onUnhandledDeepLink }: AuthDeps) {
  const microsoftDeviceCode = ref<DeviceCodeResponse | null>(null);
  const atlasDeviceCode = ref<DeviceCodeResponse | null>(null);
  const profile = ref<Profile | null>(null);
  const atlasProfile = ref<AtlasProfile | null>(null);
  const launcherLinkSession = ref<LauncherLinkSession | null>(null);
  const authInFlight = ref(false);
  const atlasAuthInFlight = ref(false);
  let microsoftPkceLoginAttempt = 0;
  let microsoftDeviceLoginAttempt = 0;
  let atlasDeviceLoginAttempt = 0;
  let launcherLinkPollAttempt = 0;
  let launcherLinkPollTimer: number | undefined;
  const LAUNCHER_LINK_STORAGE_KEY = "atlas.launcherLinkSession";
  const LAUNCHER_LINK_POLL_INTERVAL_MS = 3000;

  const isSigningIn = computed(
    () =>
      authInFlight.value ||
      atlasAuthInFlight.value ||
      !!microsoftDeviceCode.value ||
      !!atlasDeviceCode.value
  );

  function hasSignInInFlight() {
    return (
      authInFlight.value ||
      atlasAuthInFlight.value ||
      !!microsoftDeviceCode.value ||
      !!atlasDeviceCode.value
    );
  }

  async function focusLauncherWindow() {
    try {
      await invoke("focus_main_window");
      return;
    } catch (err) {
      pushLog(`Backend window focus failed, using frontend fallback: ${String(err)}`);
    }

    try {
      const window = getCurrentWindow();
      if (await window.isMinimized()) {
        await window.unminimize();
      }
      await window.show();
      await window.setFocus();
    } catch (err) {
      pushLog(`Failed to focus launcher window: ${String(err)}`);
    }
  }

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

  async function waitForMicrosoftDeviceApproval(deviceCodeValue: string, attempt: number) {
    try {
      const result = await invoke<Profile>("complete_device_code", {
        deviceCode: deviceCodeValue
      });
      if (attempt !== microsoftDeviceLoginAttempt) {
        return;
      }
      profile.value = result;
      microsoftDeviceCode.value = null;
      authInFlight.value = false;
      setStatus(`Signed in as ${result.name}.`);
      await focusLauncherWindow();
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
      if (attempt !== microsoftDeviceLoginAttempt) {
        return;
      }
      microsoftDeviceCode.value = null;
      authInFlight.value = false;
      setStatus(`Login failed: ${String(err)}`);
    }
  }

  async function waitForAtlasDeviceApproval(
    deviceCodeValue: string,
    intervalSeconds: number,
    attempt: number
  ) {
    try {
      const result = await invoke<AtlasProfile>("complete_atlas_device_code", {
        deviceCode: deviceCodeValue,
        intervalSeconds
      });
      if (attempt !== atlasDeviceLoginAttempt) {
        return;
      }
      atlasProfile.value = result;
      atlasDeviceCode.value = null;
      atlasAuthInFlight.value = false;
      setStatus(`Signed in to Atlas Hub as ${atlasIdentity(result)}.`);
      await focusLauncherWindow();
    } catch (err) {
      if (attempt !== atlasDeviceLoginAttempt) {
        return;
      }
      atlasDeviceCode.value = null;
      atlasAuthInFlight.value = false;
      setStatus(`Atlas login failed: ${String(err)}`);
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

  function handleDeepLink(url: string) {
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
    if (hasSignInInFlight()) {
      setStatus("A sign-in is already in progress. Finish it in your browser.");
      return;
    }
    authInFlight.value = true;
    await startMicrosoftPkceLogin();
  }

  async function startMicrosoftPkceLogin() {
    const attempt = ++microsoftPkceLoginAttempt;
    const opened = await run(async () => {
      try {
        microsoftDeviceCode.value = null;
        const authUrl = await invoke<string>("begin_deeplink_login");
        try {
          await openUrl(authUrl);
        } catch (openErr) {
          pushLog(`Microsoft PKCE browser open failed: ${String(openErr)}`);
          return false;
        }
        setStatus("Continue Microsoft sign-in in your browser.");
        return true;
      } catch (err) {
        const message = String(err);
        setStatus(`Microsoft sign-in start failed: ${message}`);
        authInFlight.value = false;
        return null;
      }
    });

    if (opened === undefined || opened === null) {
      return;
    }

    if (opened) {
      void waitForMicrosoftPkceApproval(attempt);
      return;
    }

    setStatus("Browser open failed. Falling back to device code sign-in.");
    await startMicrosoftDeviceLogin();
  }

  async function waitForMicrosoftPkceApproval(attempt: number) {
    await run(async () => {
      try {
        const result = await invoke<Profile>("complete_loopback_login");
        if (attempt !== microsoftPkceLoginAttempt) {
          return;
        }
        profile.value = result;
        authInFlight.value = false;
        setStatus(`Signed in as ${result.name}.`);
        await focusLauncherWindow();
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
        if (attempt !== microsoftPkceLoginAttempt) {
          return;
        }
        authInFlight.value = false;
        setStatus(`Microsoft sign-in failed: ${String(err)}`);
      }
    });
  }

  async function startMicrosoftDeviceLogin() {
    const attempt = ++microsoftDeviceLoginAttempt;
    const response = await run(async () => {
      try {
        microsoftDeviceCode.value = null;
        const nextDeviceCode = await invoke<DeviceCodeResponse>("start_device_code");
        microsoftDeviceCode.value = nextDeviceCode;
        const url = resolveVerificationUrl(nextDeviceCode);
        if (!url) {
          throw new Error("Microsoft device code response missing verification URL.");
        }
        if (!hasVerificationUriComplete(nextDeviceCode) && nextDeviceCode.user_code?.trim()) {
          try {
            await navigator.clipboard.writeText(nextDeviceCode.user_code);
            pushLog("Microsoft sign-in code copied to clipboard.");
          } catch {
            // Ignore clipboard errors; code is still shown in status/UI.
          }
          setStatus(
            `Continue in browser and enter code ${nextDeviceCode.user_code}.`
          );
        } else {
          setStatus("Waiting for Microsoft sign-in approval in your browser.");
        }
        await openUrl(url);
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
    void waitForMicrosoftDeviceApproval(response.device_code, attempt);
  }

  async function startAtlasLogin() {
    if (hasSignInInFlight()) {
      setStatus("A sign-in is already in progress. Finish it in your browser.");
      return;
    }
    atlasAuthInFlight.value = true;
    const attempt = ++atlasDeviceLoginAttempt;
    const response = await run(async () => {
      try {
        atlasDeviceCode.value = null;
        const nextDeviceCode = await invoke<DeviceCodeResponse>("start_atlas_device_code");
        atlasDeviceCode.value = nextDeviceCode;
        const url = resolveVerificationUrl(nextDeviceCode);
        if (!url) {
          throw new Error("Atlas device code response missing verification URL.");
        }
        await openUrl(url);
        setStatus("Waiting for Atlas sign-in approval in your browser.");
        return nextDeviceCode;
      } catch (err) {
        setStatus(`Atlas login start failed: ${String(err)}`);
        atlasAuthInFlight.value = false;
        return null;
      }
    });
    if (!response) {
      return;
    }
    void waitForAtlasDeviceApproval(response.device_code, response.interval ?? 5, attempt);
  }

  async function signOut() {
    microsoftPkceLoginAttempt += 1;
    microsoftDeviceLoginAttempt += 1;
    await run(async () => {
      try {
        await invoke("sign_out");
        profile.value = null;
        authInFlight.value = false;
        microsoftDeviceCode.value = null;
        launcherLinkSession.value = null;
        writeStoredLauncherLinkSession(null);
        setStatus("Signed out.");
      } catch (err) {
        setStatus(`Sign out failed: ${String(err)}`);
      }
    });
  }

  async function signOutAtlas() {
    atlasDeviceLoginAttempt += 1;
    await run(async () => {
      try {
        await invoke("atlas_sign_out");
        atlasProfile.value = null;
        atlasAuthInFlight.value = false;
        atlasDeviceCode.value = null;
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
    isSigningIn,
    microsoftDeviceCode,
    atlasDeviceCode,
    profile,
    atlasProfile,
    launcherLinkSession,
    restoreSession,
    restoreAtlasSession,
    restoreSessions,
    initDeepLink,
    startLogin,
    startAtlasLogin,
    signOut,
    signOutAtlas,
    createLauncherLink,
    completeLauncherLink,
    startLauncherLinkCompletionPoll
  };
}
