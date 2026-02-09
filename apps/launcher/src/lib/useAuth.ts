import { ref } from "vue";
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

export function useAuth({ setStatus, pushLog, run }: AuthDeps) {
  const authFlow = resolveAuthFlow((import.meta.env.VITE_AUTH_FLOW ?? "deeplink").toLowerCase());
  const deviceCode = ref<DeviceCodeResponse | null>(null);
  const pendingDeeplink = ref<string | null>(null);
  const profile = ref<Profile | null>(null);
  const atlasProfile = ref<AtlasProfile | null>(null);
  const atlasPendingDeeplink = ref<string | null>(null);
  const launcherLinkSession = ref<LauncherLinkSession | null>(null);
  let deviceLoginAttempt = 0;

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
      setStatus(`Signed in as ${result.name}.`);
    } catch (err) {
      if (attempt !== deviceLoginAttempt) {
        return;
      }
      deviceCode.value = null;
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
    }
  }

  async function initDeepLink() {
    const currentMicrosoft = await currentDeepLinkFor("microsoft");
    if (currentMicrosoft && authFlow !== "device_code") {
      pendingDeeplink.value = currentMicrosoft;
      await finishDeeplinkLogin(currentMicrosoft);
    }
    const currentAtlas = await currentDeepLinkFor("atlas");
    if (currentAtlas) {
      atlasPendingDeeplink.value = currentAtlas;
      await finishAtlasLogin(currentAtlas);
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
      } catch (err) {
        setStatus(`Login failed: ${String(err)}`);
      }
    });
  }

  async function startAtlasLogin() {
    await run(async () => {
      try {
        atlasPendingDeeplink.value = null;
        const authUrl = await invoke<string>("begin_atlas_login");
        await openUrl(authUrl);
        setStatus("Finish Atlas sign-in in your browser.");
      } catch (err) {
        setStatus(`Atlas login start failed: ${String(err)}`);
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
      } catch (err) {
        setStatus(`Atlas login failed: ${String(err)}`);
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
        setStatus("Launcher link code generated.");
        return session;
      } catch (err) {
        setStatus(`Launcher link failed: ${String(err)}`);
        return null;
      }
    });
    return result ?? null;
  }

  async function completeLauncherLink() {
    if (!launcherLinkSession.value || !profile.value) {
      setStatus("Missing launcher link session or Minecraft profile.");
      return false;
    }

    const session = launcherLinkSession.value;
    const result = await run(async () => {
      try {
        const completion = await invoke<LauncherLinkComplete>("complete_launcher_link_session", {
          linkSessionId: session.linkSessionId,
          proof: session.proof,
          minecraftUuid: profile.value?.id ?? "",
          minecraftName: profile.value?.name ?? "",
        });
        launcherLinkSession.value = null;
        await restoreAtlasSession();
        if (completion.warning) {
          const warning = `Launcher link completed, but ${completion.warning}`;
          setStatus(warning);
          pushLog(warning);
        } else {
          setStatus("Launcher link completed.");
        }
        return true;
      } catch (err) {
        setStatus(`Launcher link failed: ${String(err)}`);
        return false;
      }
    });

    return result === true;
  }

  return {
    authFlow,
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
    completeLauncherLink
  };
}
