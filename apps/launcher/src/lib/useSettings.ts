import { computed, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { AppSettings, InstanceConfig, ModLoaderConfig } from "@/types/settings";
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
    selectedInstanceId: null
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

  function normalizeInstance(instance: InstanceConfig, fallbackIndex: number): InstanceConfig {
    const source = instance.source === "atlas" ? "atlas" : "local";
    const atlasPack =
      source === "atlas" && instance.atlasPack
        ? {
            packId: instance.atlasPack.packId,
            packSlug: instance.atlasPack.packSlug,
            channel: instance.atlasPack.channel,
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
        instances: (loaded.instances ?? []).map((instance, index) =>
          normalizeInstance(instance, index)
        ),
        selectedInstanceId: loaded.selectedInstanceId ?? null
      };
      if (ensureDefaults()) {
        await saveSettings(true);
      }
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

    if (!settings.value.instances || settings.value.instances.length === 0) {
      const instance = createInstanceConfig("default", "Default", defaultGameDir.value);
      settings.value.instances = [instance];
      settings.value.selectedInstanceId = instance.id;
      return true;
    }

    const selected = settings.value.selectedInstanceId;
    if (!selected || !settings.value.instances.some((instance) => instance.id === selected)) {
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
    if (instances.value.length <= 1) {
      setStatus("At least one instance is required.");
      return;
    }
    const filtered = instances.value.filter((instance) => instance.id !== id);
    settings.value.instances = filtered;
    if (settings.value.selectedInstanceId === id) {
      settings.value.selectedInstanceId = filtered[0]?.id ?? null;
    }
    await saveSettings(true);
  }

  function normalizeRemoteId(packId: string) {
    return `atlas-${packId}`;
  }

  function normalizeRemoteGameDir(baseDir: string, packSlug: string, packId: string) {
    const safeSlug = packSlug
      .trim()
      .toLowerCase()
      .replace(/[^a-z0-9-]/g, "-")
      .replace(/-+/g, "-")
      .replace(/^-|-$/g, "");
    const dirId = safeSlug.length > 0 ? `atlas-${safeSlug}` : `atlas-${packId}`;
    return deriveInstanceDir(baseDir, dirId);
  }

  function createRemoteInstanceConfig(
    remote: AtlasRemotePack,
    existing: InstanceConfig | undefined,
    baseDir: string
  ): InstanceConfig {
    const id = normalizeRemoteId(remote.packId);
    const fallback = createInstanceConfig(id, remote.packName, baseDir);
    return {
      id,
      name: remote.packName,
      gameDir:
        existing?.gameDir?.trim() ||
        fallback.gameDir ||
        normalizeRemoteGameDir(baseDir, remote.packSlug, remote.packId),
      version: existing?.version ?? null,
      loader: {
        ...defaultLoader(),
        ...(existing?.loader ?? {})
      },
      javaPath: existing?.javaPath ?? "",
      memoryMb: typeof existing?.memoryMb === "number" ? existing.memoryMb : null,
      jvmArgs: (existing?.jvmArgs ?? "").trim() || null,
      source: "atlas",
      atlasPack: {
        packId: remote.packId,
        packSlug: remote.packSlug,
        channel: remote.channel,
        buildId: remote.buildId ?? null,
        buildVersion: remote.buildVersion ?? null,
        artifactKey: remote.artifactKey ?? null
      }
    };
  }

  async function syncAtlasRemotePacks(remotePacks: AtlasRemotePack[]) {
    const localInstances = instances.value.filter((instance) => instance.source !== "atlas");
    const existingRemoteByPackId = new Map<string, InstanceConfig>();
    for (const instance of instances.value) {
      if (instance.source !== "atlas" || !instance.atlasPack?.packId) {
        continue;
      }
      existingRemoteByPackId.set(instance.atlasPack.packId, instance);
    }

    const normalizedRemotePacks = [...remotePacks].sort((a, b) =>
      a.packName.localeCompare(b.packName)
    );
    const remoteInstances = normalizedRemotePacks.map((remote) =>
      createRemoteInstanceConfig(
        remote,
        existingRemoteByPackId.get(remote.packId),
        defaultGameDir.value
      )
    );

    const nextInstances = [...localInstances, ...remoteInstances];
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
    syncAtlasRemotePacks
  };
}
