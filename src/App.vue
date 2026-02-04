<script setup lang="ts">
import { onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/tauri";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/api/shell";
import Button from "./components/ui/button/Button.vue";
import Card from "./components/ui/card/Card.vue";
import CardHeader from "./components/ui/card/CardHeader.vue";
import CardTitle from "./components/ui/card/CardTitle.vue";
import CardDescription from "./components/ui/card/CardDescription.vue";
import CardContent from "./components/ui/card/CardContent.vue";
import CardFooter from "./components/ui/card/CardFooter.vue";
import Progress from "./components/ui/progress/Progress.vue";

interface DeviceCodeResponse {
  device_code: string;
  user_code: string;
  verification_uri: string;
  verification_uri_complete?: string;
  expires_in: number;
  interval: number;
  message?: string;
}

interface Profile {
  id: string;
  name: string;
}

interface LaunchEvent {
  phase: string;
  message: string;
  current?: number;
  total?: number;
  percent?: number;
}

const gameDir = ref("");
const javaPath = ref("");
const memoryMb = ref(4096);
const deviceCode = ref<DeviceCodeResponse | null>(null);
const profile = ref<Profile | null>(null);
const status = ref("Ready");
const logs = ref<string[]>([]);
const progress = ref(0);
const working = ref(false);

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
});

async function startLogin() {
  working.value = true;
  try {
    const response = await invoke<DeviceCodeResponse>("start_device_code", {
      clientId: ""
    });
    deviceCode.value = response;
    status.value = response.message ?? "Device code ready.";
    if (response.verification_uri_complete) {
      await open(response.verification_uri_complete);
    } else if (response.verification_uri) {
      await open(response.verification_uri);
    }
  } catch (err) {
    status.value = `Login start failed: ${String(err)}`;
  } finally {
    working.value = false;
  }
}

async function completeLogin() {
  if (!deviceCode.value) {
    status.value = "Start login first.";
    return;
  }
  working.value = true;
  try {
    const result = await invoke<Profile>("complete_device_code", {
      clientId: "",
      deviceCode: deviceCode.value.device_code
    });
    profile.value = result;
    status.value = `Signed in as ${result.name}.`;
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
    status.value = "Signed out.";
  } catch (err) {
    status.value = `Sign out failed: ${String(err)}`;
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
      options: {
        gameDir: gameDir.value.trim(),
        javaPath: javaPath.value.trim() || "java",
        memoryMb: Number(memoryMb.value || 4096)
      }
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
      options: {
        gameDir: gameDir.value.trim(),
        javaPath: javaPath.value.trim() || "java",
        memoryMb: Number(memoryMb.value || 4096)
      }
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
          <p class="text-sm uppercase tracking-[0.3em] text-muted-foreground">Barebones Launcher</p>
          <h1 class="mt-2 text-4xl font-semibold">MC Launchpad</h1>
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
              <Button :disabled="working" @click="startLogin">Begin sign in</Button>
              <Button variant="secondary" :disabled="working || !deviceCode" @click="completeLogin">
                I've signed in
              </Button>
              <Button v-if="profile" variant="ghost" :disabled="working" @click="signOut">
                Sign out
              </Button>
            </div>
            <div v-if="deviceCode" class="rounded-lg border border-border bg-secondary/50 p-3 text-sm">
              <div class="font-semibold">Code: <span class="mono">{{ deviceCode.user_code }}</span></div>
              <div class="text-muted-foreground">
                Use this code in your browser to approve the sign-in.
              </div>
            </div>
          </CardContent>
        </Card>

        <Card class="glass">
          <CardHeader>
            <CardTitle>Play</CardTitle>
            <CardDescription>We manage files and Java for you.</CardDescription>
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
