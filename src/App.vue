<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import AccountCard from "./components/AccountCard.vue";
import ActivityCard from "./components/ActivityCard.vue";
import InstancesCard from "./components/InstancesCard.vue";
import ModsCard from "./components/ModsCard.vue";
import PlayCard from "./components/PlayCard.vue";
import SettingsCard from "./components/SettingsCard.vue";
import SidebarNav from "./components/SidebarNav.vue";
import VersionsCard from "./components/VersionsCard.vue";
import { initLaunchEvents } from "./lib/useLaunchEvents";
import { useAuth } from "./lib/useAuth";
import { useLibrary } from "./lib/useLibrary";
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
const {
  settingsClientId,
  loadSettings,
  loadDefaultGameDir,
  saveSettings,
  instances,
  activeInstance,
  defaultGameDir,
  selectInstance,
  addInstance,
  duplicateInstance,
  updateInstance,
  removeInstance
} = useSettings({ setStatus, pushLog, run });
const {
  availableVersions,
  latestRelease,
  installedVersions,
  fabricLoaderVersions,
  mods,
  loadAvailableVersions,
  loadInstalledVersions,
  loadFabricLoaderVersions,
  loadMods,
  toggleMod,
  deleteMod
} = useLibrary({ activeInstance, setStatus, pushLog, run });
const { launchMinecraft, downloadMinecraftFiles } = useLauncher({
  profile,
  instance: activeInstance,
  setStatus,
  setProgress,
  run
});
const activeTab = ref<"library" | "settings">("library");

const modsDir = computed(() => {
  const instance = activeInstance.value;
  if (!instance) {
    return "";
  }
  const base = instance.gameDir?.trim() || defaultGameDir.value || "";
  if (!base) {
    return "";
  }
  return `${base.replace(/[\\/]+$/, "")}/mods`;
});

async function installSelectedVersion() {
  await downloadMinecraftFiles();
  await loadInstalledVersions();
}

async function refreshVersions() {
  await loadAvailableVersions();
  await loadInstalledVersions();
  await loadFabricLoaderVersions();
}

async function openModsFolder() {
  if (!modsDir.value) {
    setStatus("Mods folder is unavailable.");
    return;
  }
  try {
    const { open } = await import("@tauri-apps/plugin-opener");
    await open(modsDir.value);
  } catch (err) {
    setStatus(`Failed to open mods folder: ${String(err)}`);
  }
}

onMounted(async () => {
  await initLaunchEvents({ status, progress, pushLog });
  await restoreSession();
  await loadDefaultGameDir();
  await loadSettings();
  await initDeepLink();
  await loadAvailableVersions();
  await loadInstalledVersions();
  await loadFabricLoaderVersions();
  await loadMods();
});

watch(
  () => activeInstance.value?.id,
  async () => {
    await loadInstalledVersions();
    await loadFabricLoaderVersions();
    await loadMods();
  }
);

watch(
  () => activeInstance.value?.version,
  async () => {
    await loadFabricLoaderVersions();
  }
);

watch(
  () => activeInstance.value?.loader?.kind,
  async (kind) => {
    if (kind === "fabric") {
      await loadFabricLoaderVersions();
      return;
    }
    fabricLoaderVersions.value = [];
  }
);
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
              <InstancesCard
                :instances="instances"
                :active-instance-id="activeInstance?.id ?? null"
                :working="working"
                @select="selectInstance"
                @create="addInstance"
                @duplicate="duplicateInstance"
                @remove="removeInstance"
                @update="({ id, patch }) => updateInstance(id, patch)"
              />
              <PlayCard
                :profile="profile"
                :working="working"
                :progress="progress"
                :instance="activeInstance"
                @download="downloadMinecraftFiles"
                @launch="launchMinecraft"
              />
            </div>

            <div class="grid gap-6 lg:grid-cols-[1.2fr_1fr]">
              <VersionsCard
                :instance="activeInstance"
                :available-versions="availableVersions"
                :latest-release="latestRelease"
                :installed-versions="installedVersions"
                :fabric-loader-versions="fabricLoaderVersions"
                :working="working"
                @update="({ id, patch }) => updateInstance(id, patch)"
                @install="installSelectedVersion"
                @refresh="refreshVersions"
              />
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
            </div>

            <div class="grid gap-6 lg:grid-cols-[1.2fr_1fr]">
              <ModsCard
                :instance="activeInstance"
                :mods="mods"
                :mods-dir="modsDir"
                :working="working"
                @toggle="({ fileName, enabled }) => toggleMod(fileName, enabled)"
                @delete="deleteMod"
                @refresh="loadMods"
                @open-folder="openModsFolder"
              />
              <ActivityCard
                title="Activity"
                description="Most recent launcher events."
                :logs="logs"
              />
            </div>
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
