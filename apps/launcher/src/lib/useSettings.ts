import { computed, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { AppSettings, InstanceConfig, ModLoaderConfig } from "@/types/settings";

interface SettingsDeps {
  setStatus: (message: string) => void;
  pushLog: (entry: string) => void;
  run: <T>(task: () => Promise<T>) => Promise<T | undefined>;
}

export function useSettings({ setStatus, pushLog, run }: SettingsDeps) {
  let saveTimer: number | undefined;
  const settings = ref<AppSettings>({
    msClientId: null,
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
      memoryMb: 4096
    };
  }

  function normalizeInstance(instance: InstanceConfig, fallbackIndex: number): InstanceConfig {
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
      memoryMb: instance.memoryMb ?? 4096
    };
  }

  async function loadSettings() {
    try {
      const loaded = await invoke<AppSettings>("get_settings");
      settings.value = {
        msClientId: loaded.msClientId ?? null,
        instances: (loaded.instances ?? []).map((instance, index) =>
          normalizeInstance(instance, index)
        ),
        selectedInstanceId: loaded.selectedInstanceId ?? null
      };
      if (ensureDefaults()) {
        await saveSettings();
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

  async function saveSettings() {
    await run(async () => {
      try {
        await invoke("update_settings", {
          settings: settings.value
        });
        setStatus("Settings saved.");
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
      void saveSettings();
    }, 400);
  }

  async function updateSettings(next: AppSettings) {
    settings.value = next;
    ensureDefaults();
    await saveSettings();
  }

  async function selectInstance(id: string) {
    settings.value.selectedInstanceId = id;
    await saveSettings();
  }

  async function addInstance() {
    const id = crypto.randomUUID();
    const name = `Instance ${instances.value.length + 1}`;
    const instance = createInstanceConfig(id, name, defaultGameDir.value);
    settings.value.instances = [...instances.value, instance];
    settings.value.selectedInstanceId = id;
    await saveSettings();
  }

  async function duplicateInstance(id: string) {
    const source = instances.value.find((instance) => instance.id === id);
    if (!source) {
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
    await saveSettings();
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
    await saveSettings();
  }

  return {
    settings,
    settingsClientId,
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
    removeInstance
  };
}
