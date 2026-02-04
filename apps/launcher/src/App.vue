<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from "vue";
import AccountCard from "./components/AccountCard.vue";
import ActivityCard from "./components/ActivityCard.vue";
import GlobalProgressBar from "./components/GlobalProgressBar.vue";
import HeaderBar from "./components/HeaderBar.vue";
import InstanceView from "./components/InstanceView.vue";
import LibraryView from "./components/LibraryView.vue";
import SettingsCard from "./components/SettingsCard.vue";
import SidebarNav from "./components/SidebarNav.vue";
import { initLaunchEvents } from "./lib/useLaunchEvents";
import { useAuth } from "./lib/useAuth";
import { useLibrary } from "./lib/useLibrary";
import { useLauncher } from "./lib/useLauncher";
import { useSettings } from "./lib/useSettings";
import { useStatus } from "./lib/useStatus";
import { useWorking } from "./lib/useWorking";

const {
  status,
  logs,
  progress,
  tasks,
  pushLog,
  setStatus,
  setProgress,
  runTask,
  upsertTaskFromEvent
} = useStatus();
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
  neoforgeLoaderVersions,
  mods,
  loadAvailableVersions,
  loadInstalledVersions,
  loadFabricLoaderVersions,
  loadNeoForgeLoaderVersions,
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
const libraryView = ref<"grid" | "detail">("grid");
const accountSection = ref<HTMLElement | null>(null);

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

function openInstance(id: string) {
  selectInstance(id);
  libraryView.value = "detail";
}

function backToLibrary() {
  libraryView.value = "grid";
}

async function goToAuth() {
  activeTab.value = "settings";
  await nextTick();
  accountSection.value?.scrollIntoView({ behavior: "smooth", block: "start" });
}

async function startSignIn() {
  await startLogin();
  await goToAuth();
}

async function goToAccountSettings() {
  await goToAuth();
}

async function signOutFromMenu() {
  await signOut();
}

async function installSelectedVersion() {
  await downloadMinecraftFiles();
  await loadInstalledVersions();
}

async function refreshVersions() {
  await runTask("Refreshing versions", async () => {
    await loadAvailableVersions();
    await loadInstalledVersions();
    await loadFabricLoaderVersions();
    await loadNeoForgeLoaderVersions();
  });
}

async function refreshMods() {
  await runTask("Refreshing mods", async () => {
    await loadMods();
  });
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
  await initLaunchEvents({ status, progress, pushLog, upsertTaskFromEvent });
  await restoreSession();
  await loadDefaultGameDir();
  await loadSettings();
  await initDeepLink();
  await loadAvailableVersions();
  await loadInstalledVersions();
  await loadFabricLoaderVersions();
  await loadNeoForgeLoaderVersions();
  await loadMods();
});

watch(
  () => activeInstance.value?.id,
  async (value) => {
    if (!value) {
      libraryView.value = "grid";
      return;
    }
    await loadInstalledVersions();
    await loadFabricLoaderVersions();
    await loadNeoForgeLoaderVersions();
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
    if (kind === "neoforge") {
      await loadNeoForgeLoaderVersions();
      return;
    }
    fabricLoaderVersions.value = [];
    neoforgeLoaderVersions.value = [];
  }
);
</script>

<template>
  <div class="h-screen overflow-y-auto">
    <div class="m-4 grid h-full min-h-0 max-w-6xl gap-6 md:grid-cols-[72px_1fr] items-start">
      <SidebarNav class="max-h-screen sticky top-4 self-start" :active-tab="activeTab" @select="activeTab = $event" />

      <div class="flex h-full min-h-0 flex-col gap-6">
        <HeaderBar
          :active-tab="activeTab"
          :profile="profile"
          @sign-in="startSignIn"
          @account-settings="goToAccountSettings"
          @sign-out="signOutFromMenu"
        />

        <main class="flex-1 min-h-0 flex flex-col overflow-y-auto gap-6 pb-24">
          <section
            v-if="activeTab === 'library'"
            class="flex-1 min-h-0 flex flex-col gap-6 overflow-hidden"
          >
            <div
              v-if="libraryView === 'grid'"
              class="flex-1 min-h-0 overflow-auto"
            >
              <LibraryView
                :instances="instances"
                :active-instance-id="activeInstance?.id ?? null"
                :working="working"
                @select="openInstance"
                @create="addInstance"
              />
            </div>

            <InstanceView
              v-else
              class="flex-1 min-h-0"
              :instance="activeInstance"
              :profile="profile"
              :working="working"
              :progress="progress"
              :mods="mods"
              :mods-dir="modsDir"
              :available-versions="availableVersions"
              :latest-release="latestRelease"
              :installed-versions="installedVersions"
              :fabric-loader-versions="fabricLoaderVersions"
              :neoforge-loader-versions="neoforgeLoaderVersions"
              :instances-count="instances.length"
              @back="backToLibrary"
              @launch="launchMinecraft"
              @update-files="downloadMinecraftFiles"
              @go-to-settings="goToAuth"
              @toggle-mod="toggleMod"
              @delete-mod="deleteMod"
              @refresh-mods="refreshMods"
              @open-mods-folder="openModsFolder"
              @update-instance="({ id, patch }) => updateInstance(id, patch)"
              @install-version="installSelectedVersion"
              @refresh-versions="refreshVersions"
              @duplicate-instance="duplicateInstance"
              @remove-instance="removeInstance"
            />
          </section>

          <section v-else class="flex-1 min-h-0 overflow-auto space-y-6">
            <div class="grid gap-6 lg:grid-cols-[1.1fr_0.9fr]">
              <div ref="accountSection">
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
              <SettingsCard
                v-model:settingsClientId="settingsClientId"
                :working="working"
                @save-settings="saveSettings"
              />
            </div>
            <ActivityCard
              title="Recent activity"
              description="Helpful signals while tuning Atlas."
              :logs="logs"
            />
          </section>
        </main>
      </div>
    </div>
    <GlobalProgressBar :tasks="tasks" />
  </div>
</template>
