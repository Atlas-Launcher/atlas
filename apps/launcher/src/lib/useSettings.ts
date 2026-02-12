import { computed, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type {
  AppSettings,
  AtlasPackChannel,
  InstanceConfig,
  OnboardingIntent,
  OnboardingIntentSource,
  LaunchReadinessWizardState,
  ModLoaderConfig
} from "@/types/settings";
import type { AtlasRemotePack } from "@/types/library";

interface SettingsDeps {
  setStatus: (message: string) => void;
  pushLog: (entry: string) => void;
  run: <T>(task: () => Promise<T>) => Promise<T | undefined>;
}

export function useSettings({ setStatus, pushLog, run }: SettingsDeps) {
  let saveTimer: number | undefined;
  const settings = ref<AppSettings>({
    msClientId: null,
    atlasHubUrl: null,
    defaultMemoryMb: 4096,
    defaultJvmArgs: null,
    instances: [],
    selectedInstanceId: null,
    themeMode: "system",
    launchReadinessWizard: {
      dismissedAt: null,
      completedAt: null
    },
    pendingIntent: null,
    firstLaunchCompletedAt: null,
    firstLaunchNoticeDismissedAt: null
  });
  const defaultGameDir = ref("");

  const settingsClientId = computed({
    get: () => settings.value.msClientId ?? "",
    set: (value: string) => {
      const trimmed = value.trim();
      settings.value.msClientId = trimmed.length > 0 ? trimmed : null;
    }
  });

  const settingsAtlasHubUrl = computed({
    get: () => settings.value.atlasHubUrl ?? "",
    set: (value: string) => {
      const trimmed = value.trim().replace(/\/+$/, "");
      settings.value.atlasHubUrl = trimmed.length > 0 ? trimmed : null;
    }
  });

  const settingsThemeMode = computed({
    get: () => settings.value.themeMode ?? "system",
    set: (value: "light" | "dark" | "system") => {
      settings.value.themeMode = value;
      applyTheme(value);
      queueSave();
    }
  });

  const settingsDefaultMemoryMb = computed({
    get: () => settings.value.defaultMemoryMb ?? 4096,
    set: (value: number | string) => {
      const parsed = Number(value);
      if (!Number.isFinite(parsed)) {
        settings.value.defaultMemoryMb = 4096;
        return;
      }
      settings.value.defaultMemoryMb = Math.max(1024, Math.round(parsed));
    }
  });

  const settingsDefaultJvmArgs = computed({
    get: () => settings.value.defaultJvmArgs ?? "",
    set: (value: string) => {
      const trimmed = value.trim();
      settings.value.defaultJvmArgs = trimmed.length > 0 ? trimmed : null;
    }
  });

  const instances = computed(() => settings.value.instances ?? []);
  const activeInstance = computed(() => {
    const selected = settings.value.selectedInstanceId;
    return (
      instances.value.find((instance) => instance.id === selected) ??
      instances.value[0] ??
      null
    );
  });

  function deriveInstanceDir(baseDir: string, id: string) {
    if (!baseDir) {
      return "";
    }
    const trimmed = baseDir.replace(/[\\/]+$/, "");
    return `${trimmed}/instances/${id}`;
  }

  function defaultLoader(): ModLoaderConfig {
    return {
      kind: "vanilla",
      loaderVersion: null
    };
  }

  function normalizeAtlasChannel(value: string | null | undefined): AtlasPackChannel {
    if (value === "dev" || value === "beta" || value === "production") {
      return value;
    }
    return "production";
  }

  function normalizeRemoteLoaderKind(value: string | null | undefined): ModLoaderConfig["kind"] | null {
    const normalized = (value ?? "").trim().toLowerCase();
    if (!normalized) {
      return null;
    }
    if (normalized === "fabric") {
      return "fabric";
    }
    if (normalized === "neoforge" || normalized === "neo") {
      return "neoforge";
    }
    return "vanilla";
  }

  function normalizeLaunchReadinessWizard(
    value: LaunchReadinessWizardState | null | undefined
  ): LaunchReadinessWizardState {
    return {
      dismissedAt: value?.dismissedAt ?? null,
      completedAt: value?.completedAt ?? null
    };
  }

  function normalizeOnboardingSource(value: string | null | undefined): OnboardingIntentSource | null {
    if (value === "invite") {
      return "invite";
    }
    return null;
  }

  function normalizeOnboardingChannel(value: string | null | undefined): AtlasPackChannel | null {
    if (value === "dev" || value === "beta" || value === "production") {
      return value;
    }
    return null;
  }

  function normalizeOnboardingIntent(value: OnboardingIntent | null | undefined): OnboardingIntent | null {
    if (!value) {
      return null;
    }
    const source = normalizeOnboardingSource(value.source);
    const channel = normalizeOnboardingChannel(value.channel);
    const packId = value.packId?.trim();
    const createdAt = value.createdAt?.trim();
    if (!source || !channel || !packId || !createdAt) {
      return null;
    }
    return {
      source,
      channel,
      packId,
      createdAt
    };
  }

  function sameOnboardingIntent(a: OnboardingIntent | null | undefined, b: OnboardingIntent | null | undefined) {
    if (!a && !b) {
      return true;
    }
    if (!a || !b) {
      return false;
    }
    return (
      a.source === b.source &&
      a.channel === b.channel &&
      a.packId === b.packId &&
      a.createdAt === b.createdAt
    );
  }

  function createInstanceConfig(id: string, name: string, baseDir: string): InstanceConfig {
    return {
      id,
      name,
      gameDir: deriveInstanceDir(baseDir, id),
      version: null,
      loader: defaultLoader(),
      javaPath: "",
      memoryMb: null,
      jvmArgs: null,
      source: "local",
      atlasPack: null
    };
  }

  function isAtlasBackedInstance(instance: InstanceConfig) {
    return !!instance.atlasPack?.packId?.trim() || instance.source === "atlas";
  }

  function normalizeInstance(instance: InstanceConfig, fallbackIndex: number): InstanceConfig {
    const source = isAtlasBackedInstance(instance) ? "atlas" : "local";
    const atlasPack =
      source === "atlas" && instance.atlasPack
        ? {
          packId: instance.atlasPack.packId,
          packSlug: instance.atlasPack.packSlug,
          channel: normalizeAtlasChannel(instance.atlasPack.channel),
          buildId: instance.atlasPack.buildId ?? null,
          buildVersion: instance.atlasPack.buildVersion ?? null,
          artifactKey: instance.atlasPack.artifactKey ?? null
        }
        : null;
    return {
      id: instance.id || `instance-${fallbackIndex}`,
      name: instance.name || `Instance ${fallbackIndex + 1}`,
      gameDir: instance.gameDir ?? "",
      version: instance.version ?? null,
      loader: {
        ...defaultLoader(),
        ...(instance.loader ?? {})
      },
      javaPath: instance.javaPath ?? "",
      memoryMb: typeof instance.memoryMb === "number" ? instance.memoryMb : null,
      jvmArgs: (instance.jvmArgs ?? "").trim() || null,
      source,
      atlasPack
    };
  }

  function dedupeInstances(input: InstanceConfig[]) {
    const byInstanceId = new Map<string, InstanceConfig>();
    const atlasByPackId = new Map<string, string>();
    const output: InstanceConfig[] = [];

    for (const instance of input) {
      if (byInstanceId.has(instance.id)) {
        continue;
      }

      if (instance.source === "atlas" && instance.atlasPack?.packId) {
        const packId = instance.atlasPack.packId.trim();
        if (!packId) {
          continue;
        }
        if (atlasByPackId.has(packId)) {
          continue;
        }
        atlasByPackId.set(packId, instance.id);
      }

      byInstanceId.set(instance.id, instance);
      output.push(instance);
    }

    return output;
  }

  async function loadSettings() {
    try {
      const loaded = await invoke<AppSettings>("get_settings");
      settings.value = {
        msClientId: loaded.msClientId ?? null,
        atlasHubUrl: loaded.atlasHubUrl ?? null,
        defaultMemoryMb:
          typeof loaded.defaultMemoryMb === "number" && Number.isFinite(loaded.defaultMemoryMb)
            ? Math.max(1024, Math.round(loaded.defaultMemoryMb))
            : 4096,
        defaultJvmArgs: (loaded.defaultJvmArgs ?? "").trim() || null,
        instances: dedupeInstances(
          (loaded.instances ?? []).map((instance, index) =>
            normalizeInstance(instance, index)
          )
        ),
        selectedInstanceId: loaded.selectedInstanceId ?? null,
        themeMode: (loaded.themeMode as "light" | "dark" | "system") ?? null,
        launchReadinessWizard: normalizeLaunchReadinessWizard(loaded.launchReadinessWizard),
        pendingIntent: normalizeOnboardingIntent(loaded.pendingIntent),
        firstLaunchCompletedAt: loaded.firstLaunchCompletedAt ?? null,
        firstLaunchNoticeDismissedAt: loaded.firstLaunchNoticeDismissedAt ?? null
      };
      if (ensureDefaults()) {
        await saveSettings(true);
      }
      applyTheme(settings.value.themeMode ?? "system");
    } catch (err) {
      pushLog(`Failed to load settings: ${String(err)}`);
    }
  }

  async function loadDefaultGameDir() {
    try {
      defaultGameDir.value = await invoke<string>("get_default_game_dir");
    } catch (err) {
      pushLog(`Failed to resolve default game dir: ${String(err)}`);
    }
  }

  function ensureDefaults() {
    let changed = false;

    const defaultMemory = settings.value.defaultMemoryMb;
    if (!defaultMemory || defaultMemory < 1024) {
      settings.value.defaultMemoryMb = 4096;
      changed = true;
    }
    // Do not auto-create a default local profile. Start with zero instances if none exist.
    if (!settings.value.instances) {
      settings.value.instances = [];
      changed = true;
    }

    if (!settings.value.launchReadinessWizard) {
      settings.value.launchReadinessWizard = normalizeLaunchReadinessWizard(null);
      changed = true;
    }

    const normalizedIntent = normalizeOnboardingIntent(settings.value.pendingIntent);
    if (!sameOnboardingIntent(normalizedIntent, settings.value.pendingIntent)) {
      settings.value.pendingIntent = normalizedIntent;
      changed = true;
    }

    if (settings.value.firstLaunchCompletedAt === undefined) {
      settings.value.firstLaunchCompletedAt = null;
      changed = true;
    }

    if (settings.value.firstLaunchNoticeDismissedAt === undefined) {
      settings.value.firstLaunchNoticeDismissedAt = null;
      changed = true;
    }

    const selected = settings.value.selectedInstanceId;
    if (!selected || !settings.value.instances.some((instance) => instance.id === selected)) {
      // If there are instances, prefer the first. Otherwise leave as null instead of creating one.
      settings.value.selectedInstanceId = settings.value.instances[0]?.id ?? null;
      changed = true;
    }
    return changed;
  }

  async function saveSettings(silent = false) {
    await run(async () => {
      try {
        await invoke("update_settings", {
          settings: settings.value
        });
        if (!silent) {
          setStatus("Settings saved.");
        }
      } catch (err) {
        setStatus(`Settings save failed: ${String(err)}`);
      }
    });
  }

  function queueSave() {
    if (saveTimer) {
      window.clearTimeout(saveTimer);
    }
    saveTimer = window.setTimeout(() => {
      saveTimer = undefined;
      void saveSettings(true);
    }, 400);
  }

  async function updateSettings(next: AppSettings) {
    settings.value = next;
    ensureDefaults();
    await saveSettings(true);
  }

  async function selectInstance(id: string) {
    settings.value.selectedInstanceId = id;
    await saveSettings(true);
  }

  async function addInstance() {
    const id = crypto.randomUUID();
    const localCount = instances.value.filter((instance) => instance.source !== "atlas").length;
    const name = `Local Profile ${localCount + 1}`;
    const instance = createInstanceConfig(id, name, defaultGameDir.value);
    settings.value.instances = [...instances.value, instance];
    settings.value.selectedInstanceId = id;
    await saveSettings(true);
  }

  async function duplicateInstance(id: string) {
    const source = instances.value.find((instance) => instance.id === id);
    if (!source || source.source === "atlas") {
      return;
    }
    const newId = crypto.randomUUID();
    const copy: InstanceConfig = {
      ...source,
      id: newId,
      name: `${source.name} Copy`,
      gameDir: deriveInstanceDir(defaultGameDir.value, newId)
    };
    settings.value.instances = [...instances.value, copy];
    settings.value.selectedInstanceId = copy.id;
    await saveSettings(true);
  }

  async function updateInstance(id: string, patch: Partial<InstanceConfig>) {
    settings.value.instances = instances.value.map((instance) =>
      instance.id === id ? { ...instance, ...patch } : instance
    );
    queueSave();
  }

  async function removeInstance(id: string) {
    // Allow removing the last local profile; it is valid to have zero instances.
    const filtered = instances.value.filter((instance) => instance.id !== id);
    settings.value.instances = filtered;
    if (settings.value.selectedInstanceId === id) {
      settings.value.selectedInstanceId = filtered[0]?.id ?? null;
    }
    await saveSettings(true);
  }

  function normalizeRemoteId(packId: string) {
    const normalizedPackId = packId.trim();
    return `atlas-${normalizedPackId}`;
  }

  function normalizeRemoteGameDir(baseDir: string, packSlug: string, packId: string) {
    const safeSlug = packSlug
      .trim()
      .toLowerCase()
      .replace(/[^a-z0-9-]/g, "-")
      .replace(/-+/g, "-")
      .replace(/^-|-$/g, "");
    const normalizedPackId = packId.trim();
    const dirId = safeSlug.length > 0 ? `atlas-${safeSlug}` : `atlas-${normalizedPackId}`;
    return deriveInstanceDir(baseDir, dirId);
  }

  function normalizeRemotePack(remote: AtlasRemotePack): AtlasRemotePack | null {
    const packId = remote.packId?.trim();
    if (!packId) {
      return null;
    }

    return {
      ...remote,
      packId,
      packName: remote.packName?.trim() || "Atlas Pack",
      packSlug: remote.packSlug?.trim() || packId,
      channel: normalizeAtlasChannel(remote.channel),
      buildId: remote.buildId?.trim() || null,
      buildVersion: remote.buildVersion?.trim() || null,
      artifactKey: remote.artifactKey?.trim() || null,
      minecraftVersion: remote.minecraftVersion?.trim() || null,
      modloader: remote.modloader?.trim() || null,
      modloaderVersion: remote.modloaderVersion?.trim() || null,
    };
  }

  function createRemoteInstanceConfig(
    remote: AtlasRemotePack,
    existing: InstanceConfig | undefined,
    baseDir: string
  ): InstanceConfig {
    const id = normalizeRemoteId(remote.packId);
    const fallback = createInstanceConfig(id, remote.packName, baseDir);
    const selectedChannel = normalizeAtlasChannel(existing?.atlasPack?.channel ?? "production");
    const remoteChannel = normalizeAtlasChannel(remote.channel);
    const canApplyRemoteRuntime = selectedChannel === remoteChannel;
    const remoteLoaderKind = canApplyRemoteRuntime
      ? normalizeRemoteLoaderKind(remote.modloader)
      : null;
    const nextLoaderKind = remoteLoaderKind ?? existing?.loader?.kind ?? "vanilla";
    const nextLoaderVersion = canApplyRemoteRuntime
      ? remote.modloaderVersion ??
      (nextLoaderKind === "vanilla" ? null : existing?.loader?.loaderVersion ?? null)
      : existing?.loader?.loaderVersion ?? null;
    const nextMinecraftVersion = canApplyRemoteRuntime
      ? remote.minecraftVersion ?? existing?.version ?? null
      : existing?.version ?? null;
    return {
      id,
      name: remote.packName,
      gameDir:
        existing?.gameDir?.trim() ||
        fallback.gameDir ||
        normalizeRemoteGameDir(baseDir, remote.packSlug, remote.packId),
      version: nextMinecraftVersion,
      loader: {
        kind: nextLoaderKind,
        loaderVersion: nextLoaderVersion
      },
      javaPath: existing?.javaPath ?? "",
      memoryMb: typeof existing?.memoryMb === "number" ? existing.memoryMb : null,
      jvmArgs: (existing?.jvmArgs ?? "").trim() || null,
      source: "atlas",
      atlasPack: {
        packId: remote.packId,
        packSlug: remote.packSlug,
        channel: selectedChannel,
        buildId: remote.buildId ?? null,
        buildVersion: remote.buildVersion ?? null,
        artifactKey: remote.artifactKey ?? null
      }
    };
  }

  function dedupeRemotePacks(remotePacks: AtlasRemotePack[]) {
    const byPackId = new Map<string, AtlasRemotePack>();
    for (const candidate of remotePacks) {
      const remote = normalizeRemotePack(candidate);
      if (!remote) {
        continue;
      }
      const packId = remote.packId;
      if (!byPackId.has(packId)) {
        byPackId.set(packId, remote);
        continue;
      }
      const existing = byPackId.get(packId)!;
      const existingHasBuild = !!existing.buildId;
      const nextHasBuild = !!remote.buildId;
      if (!existingHasBuild && nextHasBuild) {
        byPackId.set(packId, remote);
      }
    }
    return [...byPackId.values()];
  }

  async function syncAtlasRemotePacks(remotePacks: AtlasRemotePack[]) {
    const localInstances = instances.value.filter((instance) => !isAtlasBackedInstance(instance));
    const existingRemoteByPackId = new Map<string, InstanceConfig>();
    for (const instance of instances.value) {
      if (!instance.atlasPack?.packId) {
        continue;
      }
      existingRemoteByPackId.set(instance.atlasPack.packId.trim(), instance);
    }

    const uniqueRemotePacks = dedupeRemotePacks(remotePacks);
    const normalizedRemotePacks = [...uniqueRemotePacks].sort((a, b) =>
      a.packName.localeCompare(b.packName)
    );
    const remoteInstances = normalizedRemotePacks.map((remote) =>
      createRemoteInstanceConfig(
        remote,
        existingRemoteByPackId.get(remote.packId),
        defaultGameDir.value
      )
    );

    const nextInstances = dedupeInstances([...localInstances, ...remoteInstances]);
    const nextSelected = settings.value.selectedInstanceId;
    const hasSelected = nextInstances.some((instance) => instance.id === nextSelected);

    const previousSnapshot = JSON.stringify({
      instances: settings.value.instances ?? [],
      selectedInstanceId: settings.value.selectedInstanceId ?? null
    });
    const nextSnapshot = JSON.stringify({
      instances: nextInstances,
      selectedInstanceId: hasSelected ? nextSelected ?? null : nextInstances[0]?.id ?? null
    });

    if (previousSnapshot === nextSnapshot) {
      return;
    }

    settings.value.instances = nextInstances;
    if (!hasSelected) {
      settings.value.selectedInstanceId = nextInstances[0]?.id ?? null;
    }
    await saveSettings(true);
  }

  return {
    settings,
    settingsClientId,
    settingsAtlasHubUrl,
    settingsDefaultMemoryMb,
    settingsDefaultJvmArgs,
    loadSettings,
    loadDefaultGameDir,
    saveSettings,
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
  };
}

function applyTheme(mode: "light" | "dark" | "system") {
  const root = document.documentElement;
  const isDark =
    mode === "dark" ||
    (mode === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches);

  if (isDark) {
    root.classList.add("dark");
  } else {
    root.classList.remove("dark");
  }
}
