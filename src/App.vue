<script setup lang="ts">
import { onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrent, onOpenUrl } from "@tauri-apps/plugin-deep-link";
import { openUrl } from "@tauri-apps/plugin-opener";
import AccountCard from "./components/AccountCard.vue";
import ActivityCard from "./components/ActivityCard.vue";
import PlayCard from "./components/PlayCard.vue";
import SettingsCard from "./components/SettingsCard.vue";
import SidebarNav from "./components/SidebarNav.vue";

interface Profile {
  id: string;
  name: string;
}

interface DeviceCodeResponse {
  device_code: string;
  user_code: string;
  verification_uri: string;
  verification_uri_complete?: string;
}

interface AppSettings {
  msClientId?: string | null;
}

interface LaunchEvent {
  phase: string;
  message: string;
  current?: number;
  total?: number;
  percent?: number;
}

const authFlow = (import.meta.env.VITE_AUTH_FLOW ?? "deeplink").toLowerCase();
const deviceCode = ref<DeviceCodeResponse | null>(null);
const pendingDeeplink = ref<string | null>(null);
const manualCallbackUrl = ref("");
const profile = ref<Profile | null>(null);
const status = ref("Ready");
const logs = ref<string[]>([]);
const progress = ref(0);
const working = ref(false);
const settingsClientId = ref("");
const activeTab = ref<"library" | "settings">("library");

function pushLog(entry: string) {
  logs.value = [entry, ...logs.value].slice(0, 8);
}

onMounted(async () => {
  await listen<LaunchEvent>("launch://status", (event) => {
    const payload = event.payload;
    status.value = payload.message;
    if (typeof payload.percent === "number") {
      progress.value = payload.percent;
    } else if (payload.total && payload.current) {
      progress.value = Math.round((payload.current / payload.total) * 100);
    }
    pushLog(`${payload.phase}: ${payload.message}`);
  });

  try {
    const restored = await invoke<Profile | null>("restore_session");
    if (restored) {
      profile.value = restored;
      status.value = `Signed in as ${restored.name}.`;
    }
  } catch (err) {
    pushLog(`Failed to restore session: ${String(err)}`);
  }

  try {
    const settings = await invoke<AppSettings>("get_settings");
    settingsClientId.value = settings.msClientId ?? "";
  } catch (err) {
    pushLog(`Failed to load settings: ${String(err)}`);
  }

  if (authFlow !== "device_code") {
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
});

async function startLogin() {
  if (authFlow === "device_code") {
    await startDeviceLogin();
  } else {
    await startDeeplinkLogin();
  }
}

async function startDeviceLogin() {
  working.value = true;
  try {
    deviceCode.value = null;
    const response = await invoke<DeviceCodeResponse>("start_device_code");
    deviceCode.value = response;
    const url = response.verification_uri_complete ?? response.verification_uri;
    await openUrl(url);
    status.value = "Finish signing in, then click Complete sign-in.";
  } catch (err) {
    status.value = `Login start failed: ${String(err)}`;
  } finally {
    working.value = false;
  }
}

async function completeDeviceLogin() {
  if (!deviceCode.value) {
    status.value = "Start sign-in first.";
    return;
  }
  working.value = true;
  try {
    const result = await invoke<Profile>("complete_device_code", {
      deviceCode: deviceCode.value.device_code
    });
    profile.value = result;
    status.value = `Signed in as ${result.name}.`;
    deviceCode.value = null;
  } catch (err) {
    status.value = `Login failed: ${String(err)}`;
  } finally {
    working.value = false;
  }
}

async function startDeeplinkLogin() {
  working.value = true;
  try {
    pendingDeeplink.value = null;
    const authUrl = await invoke<string>("begin_deeplink_login");
    await openUrl(authUrl);
    status.value = "Finish signing in in your browser.";
  } catch (err) {
    status.value = `Login start failed: ${String(err)}`;
  } finally {
    working.value = false;
  }
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
    status.value = "Missing auth redirect URL. Open the atlas://auth link to continue.";
    return;
  }
  working.value = true;
  try {
    const result = await invoke<Profile>("complete_deeplink_login", {
      callbackUrl: url
    });
    profile.value = result;
    status.value = `Signed in as ${result.name}.`;
    pendingDeeplink.value = null;
    manualCallbackUrl.value = "";
  } catch (err) {
    status.value = `Login failed: ${String(err)}`;
  } finally {
    working.value = false;
  }
}

async function signOut() {
  working.value = true;
  try {
    await invoke("sign_out");
    profile.value = null;
    deviceCode.value = null;
    pendingDeeplink.value = null;
    status.value = "Signed out.";
  } catch (err) {
    status.value = `Sign out failed: ${String(err)}`;
  } finally {
    working.value = false;
  }
}

async function saveSettings() {
  working.value = true;
  try {
    const trimmed = settingsClientId.value.trim();
    await invoke("update_settings", {
      settings: {
        msClientId: trimmed.length > 0 ? trimmed : null
      }
    });
    status.value = "Settings saved.";
  } catch (err) {
    status.value = `Settings save failed: ${String(err)}`;
  } finally {
    working.value = false;
  }
}

async function launchMinecraft() {
  if (!profile.value) {
    status.value = "Sign in before launching.";
    return;
  }
  working.value = true;
  progress.value = 0;
  try {
    await invoke("launch_minecraft", {
      options: {}
    });
    status.value = "Minecraft launched.";
  } catch (err) {
    status.value = `Launch failed: ${String(err)}`;
  } finally {
    working.value = false;
  }
}

async function downloadMinecraftFiles() {
  working.value = true;
  progress.value = 0;
  try {
    await invoke("download_minecraft_files", {
      options: {}
    });
    status.value = "Minecraft files downloaded.";
  } catch (err) {
    status.value = `Download failed: ${String(err)}`;
  } finally {
    working.value = false;
  }
}
</script>

<template>
  <div class="min-h-screen px-6 py-10">
    <div class="mx-auto flex w-full max-w-6xl flex-col gap-6">
      <header class="flex items-center justify-between">
        <div>
          <p class="text-sm uppercase tracking-[0.3em] text-muted-foreground">Minecraft Launcher</p>
          <h1 class="mt-2 text-4xl font-semibold">Atlas Launcher</h1>
        </div>
        <div class="status-pill">
          <span class="status-dot"></span>
          <span>{{ status }}</span>
        </div>
      </header>

      <div class="grid gap-6 md:grid-cols-[220px_1fr]">
        <SidebarNav
          :active-tab="activeTab"
          :status="status"
          @select="activeTab = $event"
        />

        <main class="space-y-6">
          <section v-if="activeTab === 'library'" class="space-y-6">
            <div class="grid gap-6 lg:grid-cols-[1.2fr_1fr]">
              <AccountCard
                :profile="profile"
                :working="working"
                :auth-flow="authFlow"
                :device-code="deviceCode"
                :pending-deeplink="pendingDeeplink"
                v-model:manualCallbackUrl="manualCallbackUrl"
                @start-login="startLogin"
                @complete-device-login="completeDeviceLogin"
                @finish-deeplink-login="finishDeeplinkLogin"
                @sign-out="signOut"
              />
              <PlayCard
                :profile="profile"
                :working="working"
                :progress="progress"
                @download="downloadMinecraftFiles"
                @launch="launchMinecraft"
              />
            </div>

            <ActivityCard
              title="Activity"
              description="Most recent launcher events."
              :logs="logs"
            />
          </section>

          <section v-else class="space-y-6">
            <SettingsCard
              v-model:settingsClientId="settingsClientId"
              :working="working"
              @save-settings="saveSettings"
            />
            <ActivityCard
              title="Recent activity"
              description="Helpful signals while tuning Atlas."
              :logs="logs"
            />
          </section>
        </main>
      </div>
    </div>
  </div>
</template>
