import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getCurrent, onOpenUrl } from "@tauri-apps/plugin-deep-link";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { AuthFlow, DeviceCodeResponse, Profile } from "@/types/auth";

interface AuthDeps {
  setStatus: (message: string) => void;
  pushLog: (entry: string) => void;
  run: <T>(task: () => Promise<T>) => Promise<T | undefined>;
}

function resolveAuthFlow(value: string): AuthFlow {
  return value === "device_code" ? "device_code" : "deeplink";
}

export function useAuth({ setStatus, pushLog, run }: AuthDeps) {
  const authFlow = resolveAuthFlow((import.meta.env.VITE_AUTH_FLOW ?? "deeplink").toLowerCase());
  const deviceCode = ref<DeviceCodeResponse | null>(null);
  const pendingDeeplink = ref<string | null>(null);
  const manualCallbackUrl = ref("");
  const profile = ref<Profile | null>(null);

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

  async function initDeepLink() {
    if (authFlow === "device_code") {
      return;
    }
    try {
      const current = await getCurrent();
      if (current && current.length > 0) {
        pendingDeeplink.value = current[0];
        await finishDeeplinkLogin(current[0]);
      }
    } catch (err) {
      pushLog(`Failed to read auth redirect: ${String(err)}`);
    }

    await onOpenUrl((urls) => {
      if (!urls || urls.length === 0) {
        return;
      }
      const url = urls[0];
      pendingDeeplink.value = url;
      pushLog(`Auth redirect received: ${url}`);
      void finishDeeplinkLogin(url);
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
    await run(async () => {
      try {
        deviceCode.value = null;
        const response = await invoke<DeviceCodeResponse>("start_device_code");
        deviceCode.value = response;
        const url = response.verification_uri_complete ?? response.verification_uri;
        await openUrl(url);
        setStatus("Finish signing in, then click Complete sign-in.");
      } catch (err) {
        setStatus(`Login start failed: ${String(err)}`);
      }
    });
  }

  async function completeDeviceLogin() {
    if (!deviceCode.value) {
      setStatus("Start sign-in first.");
      return;
    }
    await run(async () => {
      try {
        const result = await invoke<Profile>("complete_device_code", {
          deviceCode: deviceCode.value?.device_code ?? ""
        });
        profile.value = result;
        setStatus(`Signed in as ${result.name}.`);
        deviceCode.value = null;
      } catch (err) {
        setStatus(`Login failed: ${String(err)}`);
      }
    });
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
      try {
        const current = await getCurrent();
        if (current && current.length > 0) {
          url = current[0];
        }
      } catch (err) {
        pushLog(`Failed to read auth redirect: ${String(err)}`);
      }
    }
    if (!url && manualCallbackUrl.value.trim()) {
      url = manualCallbackUrl.value.trim();
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
        manualCallbackUrl.value = "";
      } catch (err) {
        setStatus(`Login failed: ${String(err)}`);
      }
    });
  }

  async function signOut() {
    await run(async () => {
      try {
        await invoke("sign_out");
        profile.value = null;
        deviceCode.value = null;
        pendingDeeplink.value = null;
        setStatus("Signed out.");
      } catch (err) {
        setStatus(`Sign out failed: ${String(err)}`);
      }
    });
  }

  return {
    authFlow,
    profile,
    deviceCode,
    pendingDeeplink,
    manualCallbackUrl,
    restoreSession,
    initDeepLink,
    startLogin,
    completeDeviceLogin,
    finishDeeplinkLogin,
    signOut
  };
}
