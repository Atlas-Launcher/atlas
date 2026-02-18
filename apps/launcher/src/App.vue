<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow, ProgressBarStatus, Window } from "@tauri-apps/api/window";
import { openUrl } from "@tauri-apps/plugin-opener";
import FirstLaunchSuccessPanel from "./components/FirstLaunchSuccessPanel.vue";
import GlobalProgressBar from "./components/GlobalProgressBar.vue";
import InstanceView from "./components/InstanceView.vue";
import LaunchAssistWizard from "./components/LaunchAssistWizard.vue";
import LibraryView from "./components/LibraryView.vue";
import SettingsCard from "./components/SettingsCard.vue";
import SidebarNav from "./components/SidebarNav.vue";
import TitleBar from "./components/TitleBar.vue";
import UpdaterBanner from "./components/UpdaterBanner.vue";
import Button from "./components/ui/button/Button.vue";
import { parseOnboardingDeepLink, type OnboardingIntent } from "./lib/onboardingDeepLink";
import { initLaunchEvents } from "./lib/useLaunchEvents";
import { useAuth } from "./lib/useAuth";
import { useLibrary } from "./lib/useLibrary";
import { useLauncher } from "./lib/useLauncher";
import { useSettings } from "./lib/useSettings";
import { useStatus } from "./lib/useStatus";
import { useUpdater } from "./lib/useUpdater";
import { useWorking } from "./lib/useWorking";
import type { LaunchReadinessReport, TroubleshooterReport } from "@/types/diagnostics";
import type { AtlasPackSyncResult, AtlasRemotePack } from "@/types/library";
import type { AppSettings, InstanceConfig, ModLoaderKind } from "@/types/settings";

const {
  status,
  logs,
  progress,
  tasks,
  pushLog,
  setStatus,
  setProgress,
  runTask,
  upsertTaskFromEvent,
  latestLaunchSuccessAt
} = useStatus();
const { working, run } = useWorking();
const incomingOnboardingIntent = ref<OnboardingIntent | null>(null);
const {
  isSigningIn,
  microsoftDeviceCode,
  atlasDeviceCode,
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
} = useAuth({
  setStatus,
  pushLog,
  run,
  onUnhandledDeepLink: (url) => {
    const intent = parseOnboardingDeepLink(url);
    if (intent) {
      incomingOnboardingIntent.value = intent;
    }
  }
});
const {
  settings,
  settingsClientId,
  settingsAtlasHubUrl,
  settingsDefaultMemoryMb,
  settingsMemoryMaxMb,
  settingsRecommendedMemoryMb,
  settingsSystemMemoryMb,
  settingsDefaultJvmArgs,
  loadSettings,
  loadDefaultGameDir,
  updateSettings,
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
const {
  checking: updaterChecking,
  installing: updaterInstalling,
  installComplete: updaterInstallComplete,
  showBanner: showUpdaterBanner,
  updaterBusy,
  updateInfo,
  statusText: updaterStatusText,
  errorMessage: updaterErrorMessage,
  downloadedBytes: updaterDownloadedBytes,
  totalBytes: updaterTotalBytes,
  speedBytesPerSecond: updaterSpeedBytesPerSecond,
  etaSeconds: updaterEtaSeconds,
  progressPercent: updaterProgressPercent,
  checkForUpdates,
  installUpdate,
  restartNow
} = useUpdater({ setStatus, pushLog });

const activeTab = ref<"library" | "settings">("library");
const libraryView = ref<"grid" | "detail">("grid");
const syncingRemotePacks = ref(false);
const instanceInstallStateById = ref<Record<string, boolean>>({});
const tasksPanelOpen = ref(false);
const launchReadiness = ref<LaunchReadinessReport | null>(null);
const accountStatusOpen = ref(false);
const recoveryOpen = ref(false);
// New flag: when we detect a connectivity error but required vars exist
const cannotConnect = ref(false);
const troubleshooterTrigger = ref<"settings" | "help" | "failure">("settings");
const dismissedFailureStatus = ref<string | null>(null);
const failurePromptEligible = ref(false);
const appBootstrapped = ref(false);
const HOURLY_UPDATE_CHECK_MS = 60 * 60 * 1000;
let updateCheckInterval: ReturnType<typeof setInterval> | null = null;
let failurePromptCheckVersion = 0;
const lastAutoTroubleshooterSignal = ref<string | null>(null);
const onboardingIntentApplying = ref(false);
const hubUrl = computed(() => (settingsAtlasHubUrl.value ?? "").trim() || import.meta.env.VITE_ATLAS_HUB_URL || "https://atlas.nathanm.org");
const readinessNextActionLabels: Partial<Record<string, string>> = {
  atlasLogin: "Sign in to Atlas",
  microsoftLogin: "Sign in to Microsoft",
  accountLink: "Link accounts"
};
const dismissedLaunchSuccessAt = ref<number | null>(null);

function onboardingIntentMatches(a: AppSettings["pendingIntent"], b: AppSettings["pendingIntent"]) {
  if (!a && !b) {
    return true;
  }
  if (!a || !b) {
    return false;
  }
  return (
    a.source === b.source &&
    a.packId === b.packId &&
    a.channel === b.channel &&
    a.createdAt === b.createdAt
  );
}

function normalizeUuid(value?: string | null) {
  return (value ?? "").trim().toLowerCase().replace(/-/g, "");
}

function localAccountsLinked() {
  if (!profile.value || !atlasProfile.value) {
    return false;
  }
  const atlasUuid = normalizeUuid(atlasProfile.value.mojang_uuid);
  const launcherUuid = normalizeUuid(profile.value.id);
  if (!atlasUuid || !launcherUuid) {
    return false;
  }
  return atlasUuid === launcherUuid;
}

const canLaunch = computed(() => {
  if (launchReadiness.value) {
    return (
      launchReadiness.value.atlasLoggedIn &&
      launchReadiness.value.microsoftLoggedIn &&
      launchReadiness.value.accountsLinked
    );
  }
  return localAccountsLinked();
});

const homeStatusMessage = computed(() => {
  if (launchReadiness.value) {
    if (!launchReadiness.value.microsoftLoggedIn) {
      return "Sign in with Microsoft to continue.";
    }
    if (!launchReadiness.value.atlasLoggedIn) {
      return "Sign in to your Atlas account to finish setup.";
    }
    if (!launchReadiness.value.accountsLinked) {
      return "Finish linking Minecraft in Atlas Hub before launching.";
    }
    return null;
  }

  if (!profile.value) {
    return "Sign in with Microsoft to continue.";
  }
  if (!atlasProfile.value) {
    return "Sign in to your Atlas account to finish setup.";
  }
  if (!canLaunch.value) {
    return "Finish linking Minecraft in Atlas Hub before launching.";
  }
  return null;
});
const showFirstLaunchSuccessPanel = computed(() => {
  const latest = latestLaunchSuccessAt.value;
  return !!latest && latest !== dismissedLaunchSuccessAt.value;
});
const firstLaunchPackName = computed(() => activeInstance.value?.name ?? null);

const modsDir = computed(() => {
  const base = resolveInstanceGameDir(activeInstance.value);
  if (!base) {
    return "";
  }
  return `${base.replace(/[\\/]+$/, "")}/.minecraft/mods`;
});

const troubleshooterGameDir = computed(() => activeInstanceGameDir() ?? defaultGameDir.value ?? null);
const troubleshooterPackId = computed(() =>
  activeInstance.value?.source === "atlas" ? activeInstance.value.atlasPack?.packId ?? null : null
);
const troubleshooterChannel = computed(() =>
  activeInstance.value?.source === "atlas"
    ? resolveAtlasChannel(activeInstance.value.atlasPack?.channel)
    : null
);
const statusSuggestsFailure = computed(() => {
  const value = (status.value ?? "").toLowerCase();
  if (!value.trim()) {
    return false;
  }
  return (
    value.includes("failed") ||
    value.includes("error") ||
    value.includes("corrupt") ||
    value.includes("out of memory") ||
    value.includes("java heap space") ||
    value.includes("missing")
  );
});
const recentLogsSuggestFailure = computed(() => {
  const haystack = logs.value.slice(0, 120).join("\n").toLowerCase();
  if (!haystack.trim()) {
    return false;
  }
  return (
    haystack.includes("out of memory") ||
    haystack.includes("java heap space") ||
    haystack.includes("missing minecraft version") ||
    haystack.includes("missing neoforge loader version") ||
    haystack.includes("launch failed") ||
    haystack.includes("client jar is missing") ||
    haystack.includes("pack update failed")
  );
});
const NON_ACTIONABLE_TROUBLESHOOTER_CODES = new Set([
  "atlas_not_signed_in",
  "microsoft_not_signed_in",
  "account_link_mismatch"
]);
const troubleshooterBlockedByReadiness = computed(
  () => !launchReadiness.value || !launchReadiness.value.readyToLaunch
);
const showTroubleshooterFailurePrompt = computed(
  () =>
    !troubleshooterBlockedByReadiness.value &&
    failurePromptEligible.value &&
    (statusSuggestsFailure.value || recentLogsSuggestFailure.value) &&
    dismissedFailureStatus.value !== status.value
);

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

function activeInstanceGameDir(): string | null {
  const gameDir = resolveInstanceGameDir(activeInstance.value);
  return gameDir.trim() ? gameDir : null;
}

async function refreshLaunchReadiness(options?: { autoOpen?: boolean }) {
  try {
    launchReadiness.value = await invoke<LaunchReadinessReport>("get_launch_readiness", {
      gameDir: activeInstanceGameDir()
    });
    // Clear any previous connectivity error since we successfully fetched readiness
    cannotConnect.value = false;
    const autoOpen = options?.autoOpen === true;
    if (autoOpen && launchReadiness.value && shouldAutoOpenReadinessWizard(launchReadiness.value)) {
      recoveryOpen.value = false;
      accountStatusOpen.value = true;
    }
    // If the report indicates we are not ready to launch (login/link blockers), ensure the wizard is open
    if (launchReadiness.value && hasLoginReadinessBlockers(launchReadiness.value)) {
      recoveryOpen.value = false;
      accountStatusOpen.value = true;
    }
  } catch (err) {
    const msg = String(err);
    pushLog(`Failed to load launch readiness: ${msg}`);
    // Determine whether required vars are present (hub URL or MS client ID)
    const hub = (settingsAtlasHubUrl.value ?? "").trim() || import.meta.env.VITE_ATLAS_HUB_URL || "";
    const hasMsClient = !!(settingsClientId.value ?? "").trim();
    const hasHub = !!hub.trim();

    // If we have the necessary configuration but can't reach the service, show cloud-alert state.
    if (hasMsClient || hasHub) {
      cannotConnect.value = true;
      // do not auto-open the readiness wizard in this case; the title bar will show the CloudAlert
      return;
    }

    // Otherwise surface the diagnostic error to the user and open the readiness wizard so they can set up.
    try {
      setStatus(msg);
    } catch {
      // ignore if setStatus fails for any reason
    }
    // Clear any connectivity flag and open the readiness wizard to allow configuring missing vars
    cannotConnect.value = false;
    recoveryOpen.value = false;
    accountStatusOpen.value = true;
  }
}

function shouldAutoOpenReadinessWizard(report: LaunchReadinessReport) {
  const dismissed = !!settings.value.launchReadinessWizard?.dismissedAt;
  const completed = !!settings.value.launchReadinessWizard?.completedAt;
  return hasLoginReadinessBlockers(report) && !dismissed && !completed;
}

function hasLoginReadinessBlockers(report: LaunchReadinessReport) {
  return !report.atlasLoggedIn || !report.microsoftLoggedIn || !report.accountsLinked;
}

async function persistIncomingOnboardingIntent(intent: OnboardingIntent) {
  if (onboardingIntentMatches(settings.value.pendingIntent, intent)) {
    return;
  }
  const nextSettings = {
    ...settings.value,
    pendingIntent: intent
  };
  await updateSettings(nextSettings);
  setStatus("Invite handoff received. Preparing your pack in Atlas Launcher.");
}

function findAtlasInstanceByPackId(packId: string) {
  return instances.value.find(
    (instance) => instance.source === "atlas" && instance.atlasPack?.packId === packId
  ) ?? null;
}

async function applyPendingOnboardingIntent() {
  const intent = settings.value.pendingIntent;
  if (!intent || onboardingIntentApplying.value) {
    return false;
  }

  const matchedInstance = findAtlasInstanceByPackId(intent.packId);
  if (!matchedInstance) {
    return false;
  }

  onboardingIntentApplying.value = true;
  try {
    if (matchedInstance.atlasPack && matchedInstance.atlasPack.channel !== intent.channel) {
      await updateInstance(matchedInstance.id, {
        atlasPack: {
          ...matchedInstance.atlasPack,
          channel: intent.channel,
          buildId: null,
          buildVersion: null,
          artifactKey: null
        }
      });
    }

    if (settings.value.selectedInstanceId !== matchedInstance.id) {
      await selectInstance(matchedInstance.id);
    }

    activeTab.value = "library";
    libraryView.value = "detail";
    await refreshLaunchReadiness();
    if (launchReadiness.value && hasLoginReadinessBlockers(launchReadiness.value)) {
      recoveryOpen.value = false;
      accountStatusOpen.value = true;
    }
    setStatus(`Invite ready for ${matchedInstance.name}. Press Play to continue.`);

    const nextSettings = {
      ...settings.value,
      pendingIntent: null
    };
    await updateSettings(nextSettings);
    return true;
  } finally {
    onboardingIntentApplying.value = false;
  }
}

async function persistReadinessWizardState(state: { dismissedAt?: string | null; completedAt?: string | null }) {
  const nextSettings = {
    ...settings.value,
    launchReadinessWizard: {
      dismissedAt: state.dismissedAt ?? settings.value.launchReadinessWizard?.dismissedAt ?? null,
      completedAt: state.completedAt ?? settings.value.launchReadinessWizard?.completedAt ?? null
    }
  };
  await updateSettings(nextSettings);
}

function buildDefaultLauncherSettings(): AppSettings {
  const base = (defaultGameDir.value ?? "").trim().replace(/[\\/]+$/, "");
  return {
    msClientId: null,
    atlasHubUrl: null,
    defaultMemoryMb: 4096,
    defaultJvmArgs: null,
    instances: [
      {
        id: "default",
        name: "Default",
        gameDir: base ? `${base}/instances/default` : "",
        version: null,
        loader: {
          kind: "vanilla",
          loaderVersion: null
        },
        javaPath: "",
        memoryMb: null,
        jvmArgs: null,
        source: "local",
        atlasPack: null
      }
    ],
    selectedInstanceId: "default",
    themeMode: "system",
    launchReadinessWizard: {
      dismissedAt: null,
      completedAt: null
    },
    pendingIntent: null,
    firstLaunchCompletedAt: null,
    firstLaunchNoticeDismissedAt: null,
    defaultMemoryProfileV1Applied: false
  };
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

async function openReadinessWizard() {
  await refreshLaunchReadiness();
  recoveryOpen.value = false;
  accountStatusOpen.value = true;
}

function dismissTroubleshooterFailurePrompt() {
  dismissedFailureStatus.value = status.value;
}

async function openTroubleshooter(trigger: "settings" | "help" | "failure") {
  pushLog(`[LaunchAssist] open recovery requested (trigger=${trigger})`);
  await refreshLaunchReadiness();
  pushLog(`[LaunchAssist] launchReadiness=${JSON.stringify(launchReadiness.value)}`);
  const readiness = launchReadiness.value;
  // Only block opening Launch Assist recovery automatically for failure-triggered auto-open when readiness is not ready.
  if (trigger === "failure" && (!readiness || !readiness.readyToLaunch)) {
    const blocker = readiness?.checklist.find((item) => !item.ready)?.label;
    setStatus(
      blocker
        ? `Complete account status check first: ${blocker}.`
        : "Complete account status checks before opening Launch Assist."
    );
    pushLog(`[LaunchAssist] blocked auto-open due to readiness; trigger=${trigger}; blocker=${blocker ?? "none"}`);
    recoveryOpen.value = false;
    accountStatusOpen.value = true;
    if (trigger === "failure") {
      dismissTroubleshooterFailurePrompt();
    }
    return;
  }

  // For manual opens (help/settings) we allow recovery mode even if readiness isn't fully ready.
  troubleshooterTrigger.value = trigger;
  accountStatusOpen.value = false;
  recoveryOpen.value = true;
  pushLog(`[LaunchAssist] opened recovery mode (trigger=${trigger})`);
  if (trigger === "failure") {
    dismissTroubleshooterFailurePrompt();
  }
}

function handleTroubleshooterStatus(message: string) {
  setStatus(message);
}

function handleTroubleshooterLog(message: string) {
  pushLog(`[LaunchAssist:${troubleshooterTrigger.value}] ${message}`);
}

async function refreshFailurePromptEligibility() {
  if (
    (!statusSuggestsFailure.value && !recentLogsSuggestFailure.value) ||
    troubleshooterBlockedByReadiness.value
  ) {
    failurePromptEligible.value = false;
    return;
  }

  const checkVersion = ++failurePromptCheckVersion;
  try {
    const report = await invoke<TroubleshooterReport>("run_troubleshooter", {
      gameDir: troubleshooterGameDir.value,
      recentStatus: status.value || null,
      recentLogs: logs.value.slice(0, 120)
    });
    if (checkVersion !== failurePromptCheckVersion) {
      return;
    }
    failurePromptEligible.value = report.findings.some(
      (finding) => !NON_ACTIONABLE_TROUBLESHOOTER_CODES.has(finding.code)
    );
  } catch {
    if (checkVersion !== failurePromptCheckVersion) {
      return;
    }
    failurePromptEligible.value = false;
  }
}

async function handleTroubleshooterRelinkRequested() {
  await startUnifiedAuthFlow();
}

async function openLauncherLinkPage(code: string) {
  const base = hubUrl.value.replace(/\/$/, "");
  await openUrl(`${base}/link/launcher?code=${encodeURIComponent(code)}`);
}

async function startLauncherLinking() {
  if (!profile.value) {
    setStatus("Sign in with Microsoft to continue.");
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

async function continueAccountLinking() {
  if (!atlasProfile.value) {
    await startAtlasLogin();
    return;
  }

  if (!profile.value) {
    await startMicrosoftSignIn();
    return;
  }

  if (launcherLinkSession.value) {
    setStatus("Checking account link status...");
    const completed = await completeLauncherLink();
    if (completed) {
      await refreshLaunchReadiness();
      return;
    }

    const activeSession = launcherLinkSession.value;
    if (activeSession) {
      await openLauncherLinkPage(activeSession.linkCode);
      startLauncherLinkCompletionPoll(activeSession);
    }
    await refreshLaunchReadiness();
    return;
  }

  await startLauncherLinking();
  await refreshLaunchReadiness();
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
    setStatus("Sign in to your Atlas account to update this profile.");
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
  await refreshLaunchReadiness();

  if (!canLaunch.value) {
    setStatus("Install complete. Finish linking Minecraft in Atlas Hub before launching.");
    return;
  }

  await launchMinecraft();
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
    await openUrl(modsDir.value)
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
      await refreshLaunchReadiness();
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
    const onboardingHandled = await applyPendingOnboardingIntent();
    if (onboardingHandled) {
      return;
    }
    setStatus(`Atlas packs synced (${remotePacks.length}).`);
  } catch (err) {
    setStatus(`Failed to sync Atlas packs: ${String(err)}`);
  } finally {
    syncingRemotePacks.value = false;
  }
}

async function checkForUpdatesFromSettings() {
  await checkForUpdates({ userInitiated: true });
}

async function installLauncherUpdate() {
  await installUpdate();
}

async function runStartupUpdateInstall() {
  const available = await checkForUpdates();
  if (!available) {
    return false;
  }
  const installed = await installUpdate();
  if (!installed) {
    return false;
  }
  return await restartNow();
}

function stopHourlyUpdateChecks() {
  if (!updateCheckInterval) {
    return;
  }
  clearInterval(updateCheckInterval);
  updateCheckInterval = null;
}

function startHourlyUpdateChecks() {
  stopHourlyUpdateChecks();
  updateCheckInterval = setInterval(() => {
    void checkForUpdates();
  }, HOURLY_UPDATE_CHECK_MS);
}

async function restartLauncherAfterUpdate() {
  await restartNow();
}

async function handleReadinessAction(key: string) {
  if (isSigningIn.value) {
    setStatus("A sign-in is already in progress. Finish it in your browser.");
    return;
  }
  if (key === "atlasLogin") {
    await startAtlasLogin();
    await refreshLaunchReadiness();
    return;
  }
  if (key === "microsoftLogin") {
    await startMicrosoftSignIn();
    await refreshLaunchReadiness();
    return;
  }
  if (key === "accountLink") {
    await continueAccountLinking();
    return;
  }
}

async function dismissReadinessWizard() {
  accountStatusOpen.value = false;
  await persistReadinessWizardState({
    dismissedAt: new Date().toISOString()
  });
}

async function completeReadinessWizard() {
  accountStatusOpen.value = false;
  await persistReadinessWizardState({
    completedAt: new Date().toISOString(),
    dismissedAt: null
  });
}

async function closeAccountStatus() {
  if (launchReadiness.value?.readyToLaunch) {
    accountStatusOpen.value = false;
    return;
  }
  await dismissReadinessWizard();
}

async function retryLaunchFromAssist() {
  await launchActiveInstance();
}

function markFirstLaunchSuccessNoticeDismissed() {
  dismissedLaunchSuccessAt.value = latestLaunchSuccessAt.value;
}

async function openLaunchAssistRecoveryFromSuccess() {
  troubleshooterTrigger.value = "help";
  accountStatusOpen.value = false;
  recoveryOpen.value = true;
}

async function handleReadinessSignOut(scope: "microsoft" | "all") {
  await runTask("Signing out", async () => {
    if (scope === "all") {
      // Always attempt to clear both Microsoft and Atlas sessions on the backend
      // regardless of local onboarding/profile state. Errors are tolerated so the
      // launcher reset flow continues.
      try {
        await signOut();
      } catch (err) {
        // signOut already sets status on failure; ignore to continue reset
      }
      try {
        await signOutAtlas();
      } catch (err) {
        // ignore to continue reset
      }

      await updateSettings(buildDefaultLauncherSettings());
      await refreshInstanceInstallStates();
      await loadInstalledVersions();
      await loadMods();
      setStatus("Signed out of Atlas and Microsoft. Launcher reset to defaults.");
    } else {
      // For microsoft-only sign out ensure we call the signOut command even if
      // there is no local profile (onboarding state) so server-side MS session
      // is cleared.
      try {
        await signOut();
      } catch (err) {
        // ignore and proceed
      }

      await persistReadinessWizardState({
        dismissedAt: null,
        completedAt: null
      });
      setStatus("Signed out of Microsoft.");
    }
    recoveryOpen.value = false;
    accountStatusOpen.value = true;
    await refreshLaunchReadiness({ autoOpen: true });
  });
}

function closeRecovery() {
  recoveryOpen.value = false;
}

async function refreshAtlasPacksFromLibrary() {
  if (!atlasProfile.value) {
    setStatus("Sign in to your Atlas account to refresh packs.");
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
  const restartedForUpdate = await runStartupUpdateInstall();
  if (restartedForUpdate) {
    return;
  }
  await restoreSessions();
  await loadDefaultGameDir();
  await loadSettings();
  await refreshLaunchReadiness({ autoOpen: true });
  await initDeepLink();
  await loadAvailableVersions();
  await loadInstalledVersions();
  await refreshInstanceInstallStates();
  await loadFabricLoaderVersions();
  await loadNeoForgeLoaderVersions();
  await loadMods();
  await syncAtlasPacks();
  startHourlyUpdateChecks();
  appBootstrapped.value = true;
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

onUnmounted(() => {
  stopHourlyUpdateChecks();
});

watch(
  () => incomingOnboardingIntent.value,
  async (intent) => {
    if (!intent) {
      return;
    }
    await persistIncomingOnboardingIntent(intent);
    incomingOnboardingIntent.value = null;
    if (atlasProfile.value) {
      await syncAtlasPacks();
    }
  }
);

watch(
  () => [
    appBootstrapped.value,
    settings.value.pendingIntent?.packId ?? null,
    settings.value.pendingIntent?.channel ?? null,
    instances.value
      .map(
        (instance) =>
          `${instance.id}:${instance.source}:${instance.atlasPack?.packId ?? ""}:${instance.atlasPack?.channel ?? ""}`
      )
      .join("|")
  ],
  async ([bootstrapped, pendingPackId]) => {
    if (!bootstrapped || !pendingPackId) {
      return;
    }
    await applyPendingOnboardingIntent();
  }
);

watch(
  () => [
    status.value,
    launchReadiness.value?.readyToLaunch ?? false,
    troubleshooterGameDir.value,
    logs.value[0] ?? ""
  ],
  async ([value, readyToLaunch]) => {
    if ((!value && !recentLogsSuggestFailure.value) || !readyToLaunch) {
      dismissedFailureStatus.value = null;
      failurePromptEligible.value = false;
      return;
    }
    await refreshFailurePromptEligibility();
  }
);

watch(
  () => [
    showTroubleshooterFailurePrompt.value,
    recentLogsSuggestFailure.value,
    status.value,
    logs.value[0] ?? ""
  ],
  async ([shouldPrompt, logTriggered, statusValue, firstLog]) => {
    if (!shouldPrompt || !logTriggered || accountStatusOpen.value || recoveryOpen.value) {
      return;
    }
    const signal = `${statusValue ?? ""}|${firstLog ?? ""}`;
    if (lastAutoTroubleshooterSignal.value === signal) {
      return;
    }
    lastAutoTroubleshooterSignal.value = signal;
    await openTroubleshooter("failure");
  }
);

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
  () => [
    profile.value?.id ?? null,
    atlasProfile.value?.id ?? null,
    atlasProfile.value?.mojang_uuid ?? null,
    activeInstance.value?.id ?? null,
    activeInstance.value?.gameDir ?? null
  ],
  async () => {
    if (!appBootstrapped.value) {
      return;
    }
    await refreshLaunchReadiness();
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
  () => latestLaunchSuccessAt.value,
  async (value) => {
    if (!value || settings.value.firstLaunchCompletedAt) {
      return;
    }
    await updateSettings({
      ...settings.value,
      firstLaunchCompletedAt: new Date(value).toISOString(),
      firstLaunchNoticeDismissedAt: null
    });
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
      :readiness="launchReadiness"
      :is-signing-in="isSigningIn"
      :cannot-connect="cannotConnect"
      :readiness-open="accountStatusOpen"
      @open-readiness-wizard="openReadinessWizard"
    />
    
    <div class="h-full grid grid-cols-[76px_1fr] gap-4 pt-11">
      <!-- SidebarNav: Floating Aside -->
      <SidebarNav
        :active-tab="activeTab"
        :tasks-count="tasks.length"
        :tasks-open="tasksPanelOpen"
        @select="activeTab = $event"
        @toggle-tasks="toggleTasksPanel"
        @open-help="openTroubleshooter('help')"
      />

      <!-- Main Content: Floating Pane -->
      <main class="flex flex-col min-h-0 overflow-visible gap-4">
        <div v-if="showUpdaterBanner" class="sticky top-0 z-[90] isolate">
          <UpdaterBanner
            :visible="showUpdaterBanner"
            :checking="updaterChecking"
            :installing="updaterInstalling"
            :install-complete="updaterInstallComplete"
            :progress-percent="updaterProgressPercent"
            :downloaded-bytes="updaterDownloadedBytes"
            :total-bytes="updaterTotalBytes"
            :speed-bytes-per-second="updaterSpeedBytesPerSecond"
            :eta-seconds="updaterEtaSeconds"
            :update-info="updateInfo"
            :error-message="updaterErrorMessage"
            @install="installLauncherUpdate"
            @restart="restartLauncherAfterUpdate"
          />
        </div>
        <FirstLaunchSuccessPanel
          :open="showFirstLaunchSuccessPanel"
          :pack-name="firstLaunchPackName"
          @retry-launch="retryLaunchFromAssist"
          @open-assist="openLaunchAssistRecoveryFromSuccess"
          @dismiss="markFirstLaunchSuccessNoticeDismissed"
        />
        <section
          v-if="activeTab === 'library'"
          class="flex-1 min-h-0 flex flex-col gap-6"
          :class="libraryView === 'detail' ? 'overflow-visible' : 'overflow-hidden'"
        >
          <div
            v-if="showTroubleshooterFailurePrompt"
            class="mx-4 rounded-2xl border border-amber-500/40 bg-amber-500/10 px-4 py-3 text-sm"
          >
            <div class="flex flex-wrap items-center justify-between gap-2">
              <div>
                <p class="font-medium text-amber-700 dark:text-amber-300">Issue detected</p>
                <p class="text-xs text-muted-foreground">
                  We detected a launch issue. Open Launch Assist for guided fixes.
                </p>
              </div>
              <div class="flex items-center gap-2">
                <Button size="sm" @click="openTroubleshooter('failure')">Open Launch Assist</Button>
                <Button size="sm" variant="outline" @click="dismissTroubleshooterFailurePrompt">Dismiss</Button>
              </div>
            </div>
          </div>
          <div
            v-if="libraryView === 'grid'"
            class="flex-1 min-h-0 overflow-visible pb-1"
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
            :memory-max-mb="settingsMemoryMaxMb"
            :recommended-memory-mb="settingsRecommendedMemoryMb"
            :system-memory-mb="settingsSystemMemoryMb"
            :default-jvm-args="settingsDefaultJvmArgs"
            @back="backToLibrary"
            @launch="launchActiveInstance"
            @update-files="installSelectedVersion"
            @toggle-mod="({ fileName, enabled }) => toggleMod(fileName, enabled)"
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
        <section v-else class="flex-1 min-h-0 overflow-hidden">
          <SettingsCard
            class="h-full min-h-0"
            v-model:settingsDefaultMemoryMb="settingsDefaultMemoryMb"
            v-model:settingsDefaultJvmArgs="settingsDefaultJvmArgs"
            v-model:settingsThemeMode="settingsThemeMode"
            :settings-memory-max-mb="settingsMemoryMaxMb"
            :settings-recommended-memory-mb="settingsRecommendedMemoryMb"
            :settings-system-memory-mb="settingsSystemMemoryMb"
            :working="working"
            :updater-busy="updaterBusy"
            :updater-status-text="updaterStatusText"
            :updater-update-version="updateInfo?.version ?? null"
            :updater-install-complete="updaterInstallComplete"
            @check-updates="checkForUpdatesFromSettings"
            @open-readiness-wizard="openReadinessWizard"
          />
        </section>
      </main>
    </div>
    <GlobalProgressBar
      v-if="tasksPanelOpen"
      :tasks="tasks"
      :pack-name="activeInstance?.name ?? null"
    />
    <LaunchAssistWizard
      :open="accountStatusOpen"
      mode="readiness"
      fixed-mode="readiness"
      :readiness="launchReadiness"
      :atlas-signed-in="!!atlasProfile"
      :microsoft-signed-in="!!profile"
      :is-signing-in="isSigningIn"
      :microsoft-device-code="microsoftDeviceCode"
      :atlas-device-code="atlasDeviceCode"
      :link-session="launcherLinkSession"
      :hub-url="hubUrl"
      :working="working"
      :next-action-labels="readinessNextActionLabels"
      :game-dir="troubleshooterGameDir"
      :pack-id="troubleshooterPackId"
      :channel="troubleshooterChannel"
      :recent-status="status"
      :recent-logs="logs"
      @action="handleReadinessAction"
      @sign-out="handleReadinessSignOut"
      @status="handleTroubleshooterStatus"
      @log="handleTroubleshooterLog"
      @relink-requested="handleTroubleshooterRelinkRequested"
      @retry-launch="retryLaunchFromAssist"
      @close="closeAccountStatus"
      @complete="completeReadinessWizard"
    />
    <LaunchAssistWizard
      :open="recoveryOpen"
      mode="recovery"
      fixed-mode="recovery"
      :readiness="launchReadiness"
      :atlas-signed-in="!!atlasProfile"
      :microsoft-signed-in="!!profile"
      :is-signing-in="isSigningIn"
      :microsoft-device-code="microsoftDeviceCode"
      :atlas-device-code="atlasDeviceCode"
      :link-session="launcherLinkSession"
      :hub-url="hubUrl"
      :working="working"
      :next-action-labels="readinessNextActionLabels"
      :game-dir="troubleshooterGameDir"
      :pack-id="troubleshooterPackId"
      :channel="troubleshooterChannel"
      :recent-status="status"
      :recent-logs="logs"
      @action="handleReadinessAction"
      @sign-out="handleReadinessSignOut"
      @status="handleTroubleshooterStatus"
      @log="handleTroubleshooterLog"
      @relink-requested="handleTroubleshooterRelinkRequested"
      @retry-launch="retryLaunchFromAssist"
      @close="closeRecovery"
      @complete="completeReadinessWizard"
    />
  </div>
</template>
