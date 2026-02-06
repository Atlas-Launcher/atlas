<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
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
import type { AtlasPackSyncResult, AtlasRemotePack } from "@/types/library";
import type { InstanceConfig, ModLoaderKind } from "@/types/settings";

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
} = useLibrary({ activeInstance, setStatus, pushLog, run, resolveGameDir: resolveInstanceGameDir });
const { launchMinecraft, downloadMinecraftFiles } = useLauncher({
  profile,
  instance: activeInstance,
  settings,
  setStatus,
  setProgress,
  run,
  resolveGameDir: resolveInstanceGameDir
});

const activeTab = ref<"library" | "settings">("library");
const libraryView = ref<"grid" | "detail">("grid");
const syncingRemotePacks = ref(false);
const instanceInstallStateById = ref<Record<string, boolean>>({});
const tasksPanelOpen = ref(false);
const TASKS_PANEL_AUTO_MINIMIZE_MS = 5000;
let tasksPanelAutoMinimizeTimer: ReturnType<typeof setTimeout> | null = null;

const modsDir = computed(() => {
  const base = resolveInstanceGameDir(activeInstance.value);
  if (!base) {
    return "";
  }
  return `${base.replace(/[\\/]+$/, "")}/.minecraft/mods`;
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

function resolveLoaderKind(modloader: string | null | undefined): ModLoaderKind {
  const normalized = (modloader ?? "").trim().toLowerCase();
  if (normalized === "fabric") {
    return "fabric";
  }
  if (normalized === "neo" || normalized === "neoforge") {
    return "neoforge";
  }
  return "vanilla";
}

function resolveLoaderKindOrNull(modloader: string | null | undefined): ModLoaderKind | null {
  const normalized = (modloader ?? "").trim().toLowerCase();
  if (!normalized) {
    return null;
  }
  return resolveLoaderKind(normalized);
}

function resolveAtlasChannel(value: string | null | undefined): "dev" | "beta" | "production" {
  if (value === "dev" || value === "beta" || value === "production") {
    return value;
  }
  return "production";
}

function resolveInstanceGameDir(instance: InstanceConfig | null): string {
  if (!instance) {
    return "";
  }
  const base = (instance.gameDir ?? "").trim().replace(/[\\/]+$/, "");
  if (!base) {
    return "";
  }
  if (instance.source !== "atlas") {
    return base;
  }
  const channel = resolveAtlasChannel(instance.atlasPack?.channel);
  return `${base}/${channel}`;
}

function normalizeVersion(value: string | null | undefined): string {
  return (value ?? "").trim().toLowerCase();
}

interface AtlasSyncOptions {
  forLaunch?: boolean;
}

async function syncAtlasInstanceFiles(instance: InstanceConfig, options?: AtlasSyncOptions) {
  if (!atlasProfile.value) {
    setStatus("Sign in to Atlas Hub to update this profile.");
    return false;
  }
  const packId = instance.atlasPack?.packId;
  if (!packId) {
    setStatus("This Atlas profile is missing pack metadata.");
    return false;
  }
  const gameDir = resolveInstanceGameDir(instance);
  if (!gameDir) {
    setStatus("Atlas profile is missing a game directory.");
    return false;
  }

  const forLaunch = options?.forLaunch === true;
  const taskLabel = forLaunch ? "Checking Atlas updates" : "Updating Atlas profile";

  try {
    await runTask(taskLabel, async () => {
      const syncPack = async () =>
        invoke<AtlasPackSyncResult>("sync_atlas_pack", {
          packId,
          gameDir,
          channel: instance.atlasPack?.channel ?? null
        });
      const hasMissingRuntimeMetadata = (value: AtlasPackSyncResult) =>
        !(value.minecraftVersion ?? "").trim() || !(value.modloader ?? "").trim();

      let result = await syncPack();
      if (hasMissingRuntimeMetadata(result)) {
        result = await syncPack();
      }

      const previousVersion = normalizeVersion(instance.version);
      const previousLoaderKind = instance.loader?.kind ?? "vanilla";
      const resultLoaderKind = resolveLoaderKindOrNull(result.modloader);
      const nextLoaderKind = resultLoaderKind ?? previousLoaderKind;
      const nextVersionValue = (result.minecraftVersion ?? "").trim()
        ? result.minecraftVersion ?? null
        : instance.version ?? null;
      const nextVersion = normalizeVersion(nextVersionValue);
      const runtimeChanged = previousVersion !== nextVersion || previousLoaderKind !== nextLoaderKind;
      const nextLoaderVersionValue = (result.modloaderVersion ?? "").trim()
        ? result.modloaderVersion ?? null
        : nextLoaderKind === "vanilla"
          ? null
          : instance.loader?.loaderVersion ?? null;

      if (forLaunch && runtimeChanged) {
        await invoke("uninstall_instance_data", {
          gameDir,
          preserveSaves: true
        });
        result = await syncPack();
        if (hasMissingRuntimeMetadata(result)) {
          result = await syncPack();
        }
      }

      const finalLoaderKind = resolveLoaderKindOrNull(result.modloader) ?? nextLoaderKind;
      const finalVersionValue = (result.minecraftVersion ?? "").trim()
        ? result.minecraftVersion ?? null
        : nextVersionValue;
      const finalLoaderVersionValue = (result.modloaderVersion ?? "").trim()
        ? result.modloaderVersion ?? null
        : finalLoaderKind === "vanilla"
          ? null
          : nextLoaderVersionValue;

      if (forLaunch) {
        if (!(finalVersionValue ?? "").trim()) {
          throw new Error("Atlas metadata is missing Minecraft version. Try update again.");
        }
        if (finalLoaderKind === "neoforge" && !(finalLoaderVersionValue ?? "").trim()) {
          throw new Error("Atlas metadata is missing NeoForge loader version. Try update again.");
        }
      }

      await updateInstance(instance.id, {
        version: finalVersionValue,
        loader: {
          kind: finalLoaderKind,
          loaderVersion: finalLoaderVersionValue
        },
        atlasPack: {
          ...(instance.atlasPack ?? {
            packId,
            packSlug: instance.name,
            channel: result.channel
          }),
          channel: result.channel,
          buildId: result.buildId ?? null,
          buildVersion: result.buildVersion ?? null
        }
      });

      await loadMods();
      if (forLaunch) {
        setStatus(
          runtimeChanged
            ? "Atlas pack runtime changed. Reinstalled files while keeping saves."
            : "Atlas pack checked for updates."
        );
      } else {
        setStatus(
          `Atlas profile updated (${result.hydratedAssets} assets, ${result.bundledFiles} files).`
        );
      }
    });
  } catch (err) {
    setStatus(`Atlas profile update failed: ${String(err)}`);
    return false;
  }
  return true;
}

async function launchActiveInstance() {
  const instance = activeInstance.value;
  if (!instance) {
    setStatus("Select a profile to launch.");
    return;
  }

  if (instance.source === "atlas") {
    const ready = await syncAtlasInstanceFiles(instance, { forLaunch: true });
    if (!ready) {
      return;
    }
    await loadInstalledVersions();
    await refreshInstanceInstallStates();
  }

  await launchMinecraft();
}

async function launchInstanceFromLibrary(id: string) {
  await selectInstance(id);
  await launchActiveInstance();
}

async function installInstanceFromLibrary(id: string) {
  await selectInstance(id);
  await installSelectedVersion();
}

async function installSelectedVersion() {
  const instance = activeInstance.value;
  if (!instance) {
    return;
  }
  if (instance.source === "atlas") {
    const ok = await syncAtlasInstanceFiles(instance);
    if (!ok) {
      return;
    }
  } else {
    await downloadMinecraftFiles();
  }
  await loadInstalledVersions();
  await refreshInstanceInstallStates();
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

function toggleTasksPanel() {
  if (!tasks.value.length) {
    return;
  }
  tasksPanelOpen.value = !tasksPanelOpen.value;
}

function clearTasksPanelAutoMinimizeTimer() {
  if (!tasksPanelAutoMinimizeTimer) {
    return;
  }
  clearTimeout(tasksPanelAutoMinimizeTimer);
  tasksPanelAutoMinimizeTimer = null;
}

function scheduleTasksPanelAutoMinimize() {
  clearTasksPanelAutoMinimizeTimer();
  tasksPanelAutoMinimizeTimer = setTimeout(() => {
    tasksPanelOpen.value = false;
    tasksPanelAutoMinimizeTimer = null;
  }, TASKS_PANEL_AUTO_MINIMIZE_MS);
}

async function updateAtlasChannel(channel: "dev" | "beta" | "production") {
  const instance = activeInstance.value;
  if (!instance || instance.source !== "atlas" || !instance.atlasPack) {
    return;
  }

  await updateInstance(instance.id, {
    atlasPack: {
      ...instance.atlasPack,
      channel,
      buildId: null,
      buildVersion: null,
      artifactKey: null
    }
  });
  await syncAtlasPacks();
  await loadInstalledVersions();
  await refreshInstanceInstallStates();
  await loadMods();
}

async function uninstallInstanceData() {
  const instance = activeInstance.value;
  if (!instance) {
    return;
  }
  await runTask("Uninstalling profile files", async () => {
    try {
      await invoke("uninstall_instance_data", {
        gameDir: resolveInstanceGameDir(instance)
      });
      await loadInstalledVersions();
      await refreshInstanceInstallStates();
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

async function refreshInstanceInstallStates() {
  const snapshot = instances.value.map((instance) => ({
    id: instance.id,
    gameDir: resolveInstanceGameDir(instance)
  }));
  if (snapshot.length === 0) {
    instanceInstallStateById.value = {};
    return;
  }

  const results = await Promise.all(
    snapshot.map(async ({ id, gameDir }) => {
      try {
        const versions = await invoke<string[]>("list_installed_versions", { gameDir });
        return [id, versions.length > 0] as const;
      } catch (err) {
        pushLog(`Failed to list installed versions for ${id}: ${String(err)}`);
        return [id, false] as const;
      }
    })
  );

  instanceInstallStateById.value = Object.fromEntries(results);
}

onMounted(async () => {
  await initLaunchEvents({ status, progress, pushLog, upsertTaskFromEvent });
  await restoreSessions();
  await loadDefaultGameDir();
  await loadSettings();
  await initDeepLink();
  await loadAvailableVersions();
  await loadInstalledVersions();
  await refreshInstanceInstallStates();
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
  () =>
    instances.value
      .map(
        (instance) =>
          `${instance.id}:${instance.gameDir ?? ""}:${instance.atlasPack?.channel ?? ""}`
      )
      .join("|"),
  async () => {
    await refreshInstanceInstallStates();
  }
);

watch(
  () => activeInstance.value?.atlasPack?.channel,
  async () => {
    await loadInstalledVersions();
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

watch(
  () => tasks.value.length,
  (count, previousCount) => {
    if (count === 0) {
      tasksPanelOpen.value = false;
      clearTasksPanelAutoMinimizeTimer();
      return;
    }
    if (previousCount === 0) {
      tasksPanelOpen.value = true;
    }
  }
);

watch(
  () => tasksPanelOpen.value,
  (open) => {
    if (open && tasks.value.length > 0) {
      scheduleTasksPanelAutoMinimize();
      return;
    }
    clearTasksPanelAutoMinimizeTimer();
  }
);

onBeforeUnmount(() => {
  clearTasksPanelAutoMinimizeTimer();
});
</script>

<template>
  <div class="h-screen overflow-hidden p-4">
    <div class="mx-auto grid h-full min-h-0 max-w-6xl gap-6 md:grid-cols-[72px_1fr] items-start">
      <SidebarNav
        class="max-h-full sticky top-0 self-start"
        :active-tab="activeTab"
        :tasks-count="tasks.length"
        :tasks-open="tasksPanelOpen"
        @select="activeTab = $event"
        @toggle-tasks="toggleTasksPanel"
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
                :instance-install-state-by-id="instanceInstallStateById"
                :working="working"
                @select="openInstance"
                @play="launchInstanceFromLibrary"
                @install="installInstanceFromLibrary"
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
              @launch="launchActiveInstance"
              @update-files="installSelectedVersion"
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
              @update-channel="updateAtlasChannel"
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
    <GlobalProgressBar
      v-if="tasks.length > 0 && tasksPanelOpen"
      :tasks="tasks"
      :pack-name="activeInstance?.name ?? null"
    />
  </div>
</template>
