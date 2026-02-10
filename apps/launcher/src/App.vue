<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow, ProgressBarStatus, Window } from "@tauri-apps/api/window";
import { openUrl } from "@tauri-apps/plugin-opener";
import ActivityCard from "./components/ActivityCard.vue";
import GlobalProgressBar from "./components/GlobalProgressBar.vue";
import InstanceView from "./components/InstanceView.vue";
import LibraryView from "./components/LibraryView.vue";
import LauncherLinkCard from "./components/LauncherLinkCard.vue";
import SettingsCard from "./components/SettingsCard.vue";
import SidebarNav from "./components/SidebarNav.vue";
import TitleBar from "./components/TitleBar.vue";
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
  isSigningIn,
  profile,
  atlasProfile,
  launcherLinkSession,
  restoreSessions,
  initDeepLink,
  startLogin,
  startAtlasLogin,
  signOut,
  signOutAtlas,
  createLauncherLink,
  completeLauncherLink,
  startLauncherLinkCompletionPoll
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
  syncAtlasRemotePacks,
  settingsThemeMode
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
const hubUrl = computed(() => (settingsAtlasHubUrl.value ?? "").trim() || import.meta.env.VITE_ATLAS_HUB_URL || "https://atlas.nathanm.org");

function normalizeUuid(value?: string | null) {
  return (value ?? "").trim().toLowerCase().replace(/-/g, "");
}

const canLaunch = computed(() => {
  if (!profile.value || !atlasProfile.value) {
    return false;
  }
  const atlasUuid = normalizeUuid(atlasProfile.value.mojang_uuid);
  const launcherUuid = normalizeUuid(profile.value.id);
  if (!atlasUuid || !launcherUuid) {
    return false;
  }
  return atlasUuid === launcherUuid;
});

const homeStatusMessage = computed(() => {
  if (!profile.value) {
    return "Sign in with Microsoft to play. Use the top-right menu to continue setup.";
  }
  if (!atlasProfile.value) {
    return "Sign in to Atlas Hub to finish setup.";
  }
  if (!canLaunch.value) {
    return "Finish linking Minecraft in Atlas Hub before launching.";
  }
  return null;
});

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

async function startUnifiedAuthFlow() {
  if (!atlasProfile.value) {
    await startAtlasLogin();
    return;
  }

  if (!profile.value) {
    await startMicrosoftSignIn();
    return;
  }

  await startLauncherLinking();
}

async function signOutMicrosoft() {
  await signOut();
}

async function signOutAtlasFromMenu() {
  await signOutAtlas();
}

async function openLauncherLinkPage(code: string) {
  const base = hubUrl.value.replace(/\/$/, "");
  await openUrl(`${base}/link/launcher?code=${encodeURIComponent(code)}`);
}

async function startLauncherLinking() {
  if (!profile.value) {
    setStatus("Sign in with Microsoft first.");
    pushLog("Launcher link start blocked: missing Microsoft profile.");
    return;
  }
  const existing = launcherLinkSession.value;
  if (existing) {
    await openLauncherLinkPage(existing.linkCode);
    pushLog("Launcher link page opened for existing session.");
    return;
  }
  const created = await createLauncherLink();
  if (created) {
    await openLauncherLinkPage(created.linkCode);
    pushLog("Launcher link page opened for new session.");
    startLauncherLinkCompletionPoll(created);
  } else {
    pushLog("Launcher link session creation failed.");
  }
}

async function completeLauncherLinkingFromMenu() {
  pushLog("Complete link clicked.");
  if (!profile.value) {
    setStatus("Sign in with Microsoft first.");
    pushLog("Complete link blocked: missing Microsoft profile.");
    return;
  }
  if (!launcherLinkSession.value) {
    await startLauncherLinking();
    return;
  }
  pushLog("Complete link: finishing launcher link session.");
  await completeLauncherLink();
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
      const forcedReinstallApplied = result.requiresFullReinstall === true;

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

      if (forLaunch && runtimeChanged && !forcedReinstallApplied) {
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
          forcedReinstallApplied
            ? "Atlas pack force-reinstall policy applied. Files were reinstalled while keeping saves."
            : runtimeChanged
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
  if (!canLaunch.value) {
    setStatus("Finish linking Minecraft in Atlas Hub before launching.");
    return;
  }
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
  if (!canLaunch.value) {
    setStatus("Finish linking Minecraft in Atlas Hub before launching.");
    return;
  }
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
  tasksPanelOpen.value = !tasksPanelOpen.value;
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
  const window = getCurrentWindow();
  try {
    await window.setProgressBar({
      status: ProgressBarStatus.Indeterminate,
      progress: 10
    });
  } catch {
    // Ignore if not running in a Tauri window.
  }
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
  try {
    const windows = await Window.getAll();
    const loading = windows.find((entry) => entry.label === "loading");
    if (loading) {
      await loading.close();
    }
    await window.setProgressBar({ status: ProgressBarStatus.None });
    await window.show();
    await window.setFocus();
  } catch {
    // Ignore if not running in a Tauri window.
  }
});

watch(
  () => [profile.value?.id ?? null, launcherLinkSession.value?.linkSessionId ?? null],
  async ([profileId, linkSessionId]) => {
    if (!profileId || !linkSessionId || !launcherLinkSession.value) {
      return;
    }
    await startLauncherLinkCompletionPoll(launcherLinkSession.value);
  }
);

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
    // Manual control only: removed auto-open logic
  }
);

</script>

<template>
  <div class="h-screen bg-transparent text-foreground overflow-hidden relative p-3">
    <!-- TitleBar overlays everything, but its internal elements will align with the p-3 grid below -->
    <TitleBar 
      :profile="profile"
      :atlas-profile="atlasProfile"
      :is-signing-in="isSigningIn"
      @sign-out-microsoft="signOutMicrosoft"
      @sign-out-atlas="signOutAtlasFromMenu"
      @start-auth-flow="startUnifiedAuthFlow"
      @complete-link="completeLauncherLinkingFromMenu"
    />
    
    <div class="h-full grid grid-cols-[76px_1fr] gap-4 pt-8">
      <!-- SidebarNav: Floating Aside -->
      <SidebarNav
        :active-tab="activeTab"
        :tasks-count="tasks.length"
        :tasks-open="tasksPanelOpen"
        @select="activeTab = $event"
        @toggle-tasks="toggleTasksPanel"
      />

      <!-- Main Content: Floating Pane -->
      <main class="flex flex-col min-h-0 overflow-visible">
        <section
          v-if="activeTab === 'library'"
          class="flex-1 min-h-0 flex flex-col gap-6 overflow-visible"
        >
          <div
            v-if="libraryView === 'grid'"
            class="flex-1 min-h-0 overflow-y-auto px-4 pr-1"
          >
            <LibraryView
              :instances="instances"
              :active-instance-id="activeInstance?.id ?? null"
              :instance-install-state-by-id="instanceInstallStateById"
              :working="working"
              :can-launch="canLaunch"
              :status-message="homeStatusMessage"
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
            :can-launch="canLaunch"
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
        <section v-else class="flex-1 min-h-0 overflow-y-auto px-4 pb-4 space-y-6">
          <LauncherLinkCard
            :link-session="launcherLinkSession"
            :atlas-profile="atlasProfile"
            :profile="profile"
            :hub-url="hubUrl"
            :working="working"
          />
          <SettingsCard
            v-model:settingsClientId="settingsClientId"
            v-model:settingsAtlasHubUrl="settingsAtlasHubUrl"
            v-model:settingsDefaultMemoryMb="settingsDefaultMemoryMb"
            v-model:settingsDefaultJvmArgs="settingsDefaultJvmArgs"
            v-model:settingsThemeMode="settingsThemeMode"
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
    <GlobalProgressBar
      v-if="tasksPanelOpen"
      :tasks="tasks"
      :pack-name="activeInstance?.name ?? null"
    />
  </div>
</template>
