<script setup lang="ts">
import { onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrent, onOpenUrl } from "@tauri-apps/plugin-deep-link";
import { openUrl } from "@tauri-apps/plugin-opener";
import Button from "./components/ui/button/Button.vue";
import Card from "./components/ui/card/Card.vue";
import CardHeader from "./components/ui/card/CardHeader.vue";
import CardTitle from "./components/ui/card/CardTitle.vue";
import CardDescription from "./components/ui/card/CardDescription.vue";
import CardContent from "./components/ui/card/CardContent.vue";
import CardFooter from "./components/ui/card/CardFooter.vue";
import Input from "./components/ui/input/Input.vue";
import Progress from "./components/ui/progress/Progress.vue";

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
const profile = ref<Profile | null>(null);
const status = ref("Ready");
const logs = ref<string[]>([]);
const progress = ref(0);
const working = ref(false);
const settingsClientId = ref("");

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
      device_code: deviceCode.value.device_code
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
  const url = callbackUrl ?? pendingDeeplink.value;
  if (!url) {
    status.value = "Missing auth redirect URL.";
    return;
  }
  working.value = true;
  try {
    const result = await invoke<Profile>("complete_deeplink_login", {
      callback_url: url
    });
    profile.value = result;
    status.value = `Signed in as ${result.name}.`;
    pendingDeeplink.value = null;
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
    <div class="mx-auto flex w-full max-w-5xl flex-col gap-6">
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

      <div class="grid gap-6 md:grid-cols-[1.2fr_1fr]">
        <Card class="glass">
          <CardHeader>
            <CardTitle>Account</CardTitle>
            <CardDescription>Connect your Microsoft account to play.</CardDescription>
          </CardHeader>
          <CardContent class="space-y-4">
            <div class="grid gap-3">
              <div class="rounded-lg border border-border bg-secondary/40 px-4 py-3 text-sm">
                <div class="text-xs uppercase tracking-widest text-muted-foreground">Status</div>
                <div class="mt-1 font-semibold text-foreground">
                  {{ profile ? `Signed in as ${profile.name}` : "Not signed in" }}
                </div>
              </div>
              <Button :disabled="working" @click="startLogin">Sign in</Button>
              <div v-if="authFlow === 'device_code' && deviceCode" class="rounded-lg border border-border bg-muted/40 px-4 py-3 text-sm">
                <div class="text-xs uppercase tracking-widest text-muted-foreground">Verification URL</div>
                <div class="mt-1 break-all text-foreground">{{ deviceCode.verification_uri }}</div>
                <div class="mt-3 text-xs uppercase tracking-widest text-muted-foreground">User Code</div>
                <div class="mt-1 text-base font-semibold text-foreground">{{ deviceCode.user_code }}</div>
              </div>
              <Button
                v-if="authFlow === 'device_code' && deviceCode"
                :disabled="working"
                variant="secondary"
                @click="completeDeviceLogin"
              >
                Complete sign-in
              </Button>
              <Button
                v-if="authFlow !== 'device_code' && pendingDeeplink && !profile"
                :disabled="working"
                variant="secondary"
                @click="finishDeeplinkLogin()"
              >
                Finish sign-in
              </Button>
              <Button v-if="profile" variant="ghost" :disabled="working" @click="signOut">
                Sign out
              </Button>
            </div>
          </CardContent>
        </Card>

        <Card class="glass">
          <CardHeader>
            <CardTitle>Play</CardTitle>
          <CardDescription>Atlas manages files and Java for you.</CardDescription>
          </CardHeader>
          <CardContent class="space-y-4">
            <div class="grid gap-3">
              <Button :disabled="working" variant="secondary" @click="downloadMinecraftFiles">
                Download game files
              </Button>
              <Button :disabled="working || !profile" @click="launchMinecraft">Launch Minecraft</Button>
            </div>
            <div class="text-xs text-muted-foreground">
              Files are stored in your app data folder and updated automatically.
            </div>
          </CardContent>
          <CardFooter>
            <div class="w-full space-y-2">
              <div class="flex items-center justify-between text-xs text-muted-foreground">
                <span>Launch progress</span>
                <span>{{ progress }}%</span>
              </div>
              <Progress :model-value="progress" />
            </div>
          </CardFooter>
        </Card>
      </div>

      <Card class="glass">
        <CardHeader>
          <CardTitle>Settings</CardTitle>
          <CardDescription>Optional sign-in overrides.</CardDescription>
        </CardHeader>
        <CardContent class="space-y-3">
          <div class="space-y-2">
            <label class="text-xs uppercase tracking-widest text-muted-foreground">
              Microsoft Client ID (optional)
            </label>
            <Input
              v-model="settingsClientId"
              placeholder="Leave blank to use the bundled client ID"
            />
          </div>
          <div class="text-xs text-muted-foreground">
            This only affects new sign-ins. Sign out and sign back in to apply.
          </div>
        </CardContent>
        <CardFooter>
          <Button :disabled="working" variant="secondary" @click="saveSettings">
            Save settings
          </Button>
        </CardFooter>
      </Card>

      <Card class="glass">
        <CardHeader>
          <CardTitle>Activity</CardTitle>
          <CardDescription>Most recent launcher events.</CardDescription>
        </CardHeader>
        <CardContent>
          <ul class="space-y-2 text-sm text-muted-foreground">
            <li v-for="(entry, index) in logs" :key="index">{{ entry }}</li>
            <li v-if="logs.length === 0">No events yet.</li>
          </ul>
        </CardContent>
      </Card>
    </div>
  </div>
</template>
