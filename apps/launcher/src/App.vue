<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
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
import type { AtlasRemotePack } from "@/types/library";

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
  profile,
  atlasProfile,
  restoreSessions,
  initDeepLink,
  startLogin,
  startAtlasLogin,
  signOut,
  signOutAtlas
} = useAuth({ setStatus, pushLog, run });
const {
  settings,
  settingsClientId,
  settingsAtlasHubUrl,
  settingsDefaultMemoryMb,
  settingsDefaultJvmArgs,
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
  removeInstance,
  syncAtlasRemotePacks
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
  settings,
  setStatus,
  setProgress,
  run
});

const activeTab = ref<"library" | "settings">("library");
const libraryView = ref<"grid" | "detail">("grid");
const syncingRemotePacks = ref(false);

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

async function startMicrosoftSignIn() {
  await startLogin();
}

async function signOutMicrosoft() {
  await signOut();
}

async function signOutAtlasFromMenu() {
  await signOutAtlas();
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

async function uninstallInstanceData() {
  const instance = activeInstance.value;
  if (!instance) {
    return;
  }
  await runTask("Uninstalling profile files", async () => {
    try {
      await invoke("uninstall_instance_data", {
        gameDir: instance.gameDir ?? ""
      });
      await loadInstalledVersions();
      await loadMods();
      setStatus("Profile files removed.");
    } catch (err) {
      setStatus(`Failed to uninstall profile data: ${String(err)}`);
    }
  });
}

async function syncAtlasPacks() {
  if (syncingRemotePacks.value) {
    return;
  }
  syncingRemotePacks.value = true;
  try {
    if (!atlasProfile.value) {
      return;
    }
    const remotePacks = await invoke<AtlasRemotePack[]>("list_atlas_remote_packs");
    await syncAtlasRemotePacks(remotePacks);
    setStatus(`Atlas packs synced (${remotePacks.length}).`);
  } catch (err) {
    setStatus(`Failed to sync Atlas packs: ${String(err)}`);
  } finally {
    syncingRemotePacks.value = false;
  }
}

async function refreshAtlasPacksFromLibrary() {
  if (!atlasProfile.value) {
    setStatus("Sign in to Atlas Hub to refresh remote packs.");
    return;
  }
  await syncAtlasPacks();
}

onMounted(async () => {
  await initLaunchEvents({ status, progress, pushLog, upsertTaskFromEvent });
  await restoreSessions();
  await loadDefaultGameDir();
  await loadSettings();
  await initDeepLink();
  await loadAvailableVersions();
  await loadInstalledVersions();
  await loadFabricLoaderVersions();
  await loadNeoForgeLoaderVersions();
  await loadMods();
  await syncAtlasPacks();
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

watch(
  () => atlasProfile.value?.id,
  async () => {
    await syncAtlasPacks();
  }
);
</script>

<template>
  <div class="h-screen overflow-hidden p-4">
    <div class="mx-auto grid h-full min-h-0 max-w-6xl gap-6 md:grid-cols-[72px_1fr] items-start">
      <SidebarNav
        class="max-h-full sticky top-0 self-start"
        :active-tab="activeTab"
        @select="activeTab = $event"
      />

      <div class="flex h-full min-h-0 flex-col gap-6">
        <HeaderBar
          :active-tab="activeTab"
          :profile="profile"
          :atlas-profile="atlasProfile"
          @sign-in-microsoft="startMicrosoftSignIn"
          @sign-out-microsoft="signOutMicrosoft"
          @sign-in-atlas="startAtlasLogin"
          @sign-out-atlas="signOutAtlasFromMenu"
        />

        <main class="flex-1 min-h-0 flex flex-col overflow-hidden gap-6">
          <section
            v-if="activeTab === 'library'"
            class="flex-1 min-h-0 flex flex-col gap-6 overflow-hidden"
          >
            <div
              v-if="libraryView === 'grid'"
              class="flex-1 min-h-0 overflow-y-auto overflow-x-hidden pr-1"
            >
              <LibraryView
                :instances="instances"
                :active-instance-id="activeInstance?.id ?? null"
                :working="working"
                @select="openInstance"
                @create="addInstance"
                @refresh-packs="refreshAtlasPacksFromLibrary"
              />
            </div>

            <InstanceView
              v-else
              class="flex-1 min-h-0"
              :instance="activeInstance"
              :profile="profile"
              :working="working"
              :mods="mods"
              :mods-dir="modsDir"
              :available-versions="availableVersions"
              :latest-release="latestRelease"
              :installed-versions="installedVersions"
              :fabric-loader-versions="fabricLoaderVersions"
              :neoforge-loader-versions="neoforgeLoaderVersions"
              :instances-count="instances.length"
              :default-memory-mb="settingsDefaultMemoryMb"
              :default-jvm-args="settingsDefaultJvmArgs"
              @back="backToLibrary"
              @launch="launchMinecraft"
              @update-files="downloadMinecraftFiles"
              @go-to-settings="startMicrosoftSignIn"
              @toggle-mod="toggleMod"
              @delete-mod="deleteMod"
              @refresh-mods="refreshMods"
              @open-mods-folder="openModsFolder"
              @update-instance="({ id, patch }) => updateInstance(id, patch)"
              @install-version="installSelectedVersion"
              @refresh-versions="refreshVersions"
              @duplicate-instance="duplicateInstance"
              @remove-instance="removeInstance"
              @uninstall-instance="uninstallInstanceData"
            />
          </section>

          <section v-else class="flex-1 min-h-0 overflow-auto space-y-6 pr-1">
            <SettingsCard
              v-model:settingsClientId="settingsClientId"
              v-model:settingsAtlasHubUrl="settingsAtlasHubUrl"
              v-model:settingsDefaultMemoryMb="settingsDefaultMemoryMb"
              v-model:settingsDefaultJvmArgs="settingsDefaultJvmArgs"
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
    <GlobalProgressBar :tasks="tasks" />
  </div>
</template>
