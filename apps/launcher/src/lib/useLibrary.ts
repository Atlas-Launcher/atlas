import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { Ref } from "vue";
import type { InstanceConfig } from "@/types/settings";
import type {
  FabricLoaderVersion,
  NeoForgeLoaderVersion,
  ModEntry,
  VersionManifestSummary,
  VersionSummary
} from "@/types/library";

interface LibraryDeps {
  activeInstance: Ref<InstanceConfig | null>;
  setStatus: (message: string) => void;
  pushLog: (entry: string) => void;
  run: <T>(task: () => Promise<T>) => Promise<T | undefined>;
  resolveGameDir: (instance: InstanceConfig | null) => string;
}

export function useLibrary({ activeInstance, setStatus, pushLog, run, resolveGameDir }: LibraryDeps) {
  const availableVersions = ref<VersionSummary[]>([]);
  const latestRelease = ref("");
  const installedVersions = ref<string[]>([]);
  const fabricLoaderVersions = ref<FabricLoaderVersion[]>([]);
  const neoforgeLoaderVersions = ref<NeoForgeLoaderVersion[]>([]);
  const mods = ref<ModEntry[]>([]);

  async function loadAvailableVersions() {
    try {
      const summary = await invoke<VersionManifestSummary>("get_version_manifest_summary");
      availableVersions.value = summary.versions;
      latestRelease.value = summary.latestRelease;
    } catch (err) {
      pushLog(`Failed to fetch versions: ${String(err)}`);
    }
  }

  async function loadInstalledVersions() {
    const instance = activeInstance.value;
    if (!instance) {
      installedVersions.value = [];
      return;
    }
    const gameDir = resolveGameDir(instance);
    if (!gameDir) {
      installedVersions.value = [];
      return;
    }
    try {
      installedVersions.value = await invoke<string[]>("list_installed_versions", {
        gameDir
      });
    } catch (err) {
      pushLog(`Failed to list installed versions: ${String(err)}`);
    }
  }

  async function loadFabricLoaderVersions() {
    const instance = activeInstance.value;
    if (!instance || instance.loader.kind !== "fabric") {
      fabricLoaderVersions.value = [];
      return;
    }
    const minecraftVersion = instance.version?.trim();
    if (!minecraftVersion) {
      fabricLoaderVersions.value = [];
      return;
    }
    try {
      fabricLoaderVersions.value = await invoke<FabricLoaderVersion[]>(
        "get_fabric_loader_versions",
        {
          minecraftVersion
        }
      );
    } catch (err) {
      pushLog(`Failed to fetch Fabric loaders: ${String(err)}`);
    }
  }

  async function loadNeoForgeLoaderVersions() {
    const instance = activeInstance.value;
    if (!instance || instance.loader.kind !== "neoforge") {
      neoforgeLoaderVersions.value = [];
      return;
    }
    try {
      neoforgeLoaderVersions.value = await invoke<NeoForgeLoaderVersion[]>(
        "get_neoforge_loader_versions"
      );
    } catch (err) {
      pushLog(`Failed to fetch NeoForge loaders: ${String(err)}`);
    }
  }

  async function loadMods() {
    const instance = activeInstance.value;
    if (!instance) {
      mods.value = [];
      return;
    }
    const gameDir = resolveGameDir(instance);
    if (!gameDir) {
      mods.value = [];
      return;
    }
    try {
      mods.value = await invoke<ModEntry[]>("list_mods", {
        gameDir
      });
    } catch (err) {
      pushLog(`Failed to list mods: ${String(err)}`);
    }
  }

  async function toggleMod(fileName: string, enabled: boolean) {
    const instance = activeInstance.value;
    if (!instance) {
      return;
    }
    const gameDir = resolveGameDir(instance);
    if (!gameDir) {
      return;
    }
    await run(async () => {
      try {
        await invoke("set_mod_enabled", {
          gameDir,
          fileName,
          enabled
        });
        await loadMods();
        setStatus(enabled ? "Mod enabled." : "Mod disabled.");
      } catch (err) {
        setStatus(`Failed to toggle mod: ${String(err)}`);
      }
    });
  }

  async function deleteMod(fileName: string) {
    const instance = activeInstance.value;
    if (!instance) {
      return;
    }
    const gameDir = resolveGameDir(instance);
    if (!gameDir) {
      return;
    }
    await run(async () => {
      try {
        await invoke("delete_mod", {
          gameDir,
          fileName
        });
        await loadMods();
        setStatus("Mod deleted.");
      } catch (err) {
        setStatus(`Failed to delete mod: ${String(err)}`);
      }
    });
  }

  return {
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
  };
}
