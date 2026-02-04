<script setup lang="ts">
import { onMounted, ref } from "vue";
import AccountCard from "./components/AccountCard.vue";
import ActivityCard from "./components/ActivityCard.vue";
import PlayCard from "./components/PlayCard.vue";
import SettingsCard from "./components/SettingsCard.vue";
import SidebarNav from "./components/SidebarNav.vue";
import { initLaunchEvents } from "./lib/useLaunchEvents";
import { useAuth } from "./lib/useAuth";
import { useLauncher } from "./lib/useLauncher";
import { useSettings } from "./lib/useSettings";
import { useStatus } from "./lib/useStatus";
import { useWorking } from "./lib/useWorking";

const { status, logs, progress, pushLog, setStatus, setProgress } = useStatus();
const { working, run } = useWorking();
const {
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
} = useAuth({ setStatus, pushLog, run });
const { settingsClientId, loadSettings, saveSettings } = useSettings({ setStatus, pushLog, run });
const { launchMinecraft, downloadMinecraftFiles } = useLauncher({
  profile,
  setStatus,
  setProgress,
  run
});
const activeTab = ref<"library" | "settings">("library");

function pushLog(entry: string) {
  logs.value = [entry, ...logs.value].slice(0, 8);
}

onMounted(async () => {
  await initLaunchEvents({ status, progress, pushLog });
  await restoreSession();
  await loadSettings();
  await initDeepLink();
});
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
