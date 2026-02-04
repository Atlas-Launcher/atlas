<script setup lang="ts">
import { onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/tauri";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/api/shell";
import Button from "./components/ui/button/Button.vue";
import Input from "./components/ui/input/Input.vue";
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

const clientId = ref(localStorage.getItem("mc_client_id") ?? "");
const gameDir = ref("");
const javaPath = ref("java");
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
  try {
    const dir = await invoke<string>("get_default_game_dir");
    gameDir.value = dir;
  } catch (err) {
    pushLog(`Failed to read default game dir: ${String(err)}`);
  }

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
});

async function startLogin() {
  if (!clientId.value.trim()) {
    status.value = "Enter your Microsoft client ID first.";
    return;
  }
  working.value = true;
  try {
    localStorage.setItem("mc_client_id", clientId.value.trim());
    const response = await invoke<DeviceCodeResponse>("start_device_code", {
      clientId: clientId.value.trim()
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
      clientId: clientId.value.trim(),
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
            <CardTitle>Sign in</CardTitle>
            <CardDescription>Device code login for Microsoft accounts.</CardDescription>
          </CardHeader>
          <CardContent class="space-y-4">
            <div>
              <label class="text-xs uppercase tracking-widest text-muted-foreground">Microsoft Client ID</label>
              <Input v-model="clientId" placeholder="GUID from your Azure app" class="mt-2" />
            </div>
            <div class="grid gap-3">
              <Button :disabled="working" @click="startLogin">Start device login</Button>
              <Button variant="secondary" :disabled="working || !deviceCode" @click="completeLogin">
                Complete login
              </Button>
            </div>
            <div v-if="deviceCode" class="rounded-lg border border-border bg-secondary/50 p-3 text-sm">
              <div class="font-semibold">Code: <span class="mono">{{ deviceCode.user_code }}</span></div>
              <div class="text-muted-foreground">{{ deviceCode.verification_uri }}</div>
            </div>
          </CardContent>
        </Card>

        <Card class="glass">
          <CardHeader>
            <CardTitle>Launch settings</CardTitle>
            <CardDescription>Point at a Minecraft folder and JVM.</CardDescription>
          </CardHeader>
          <CardContent class="space-y-4">
            <div>
              <label class="text-xs uppercase tracking-widest text-muted-foreground">Game directory</label>
              <Input v-model="gameDir" class="mt-2" />
            </div>
            <div class="grid gap-4 sm:grid-cols-2">
              <div>
                <label class="text-xs uppercase tracking-widest text-muted-foreground">Java path</label>
                <Input v-model="javaPath" class="mt-2" />
              </div>
              <div>
                <label class="text-xs uppercase tracking-widest text-muted-foreground">Memory (MB)</label>
                <Input v-model.number="memoryMb" type="number" min="1024" step="256" class="mt-2" />
              </div>
            </div>
            <Button :disabled="working || !profile" @click="launchMinecraft">Launch Minecraft</Button>
            <div v-if="profile" class="text-sm text-muted-foreground">
              Signed in as <span class="font-semibold text-foreground">{{ profile.name }}</span>
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
