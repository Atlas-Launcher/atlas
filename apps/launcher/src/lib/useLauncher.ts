import { invoke } from "@tauri-apps/api/core";
import type { Ref } from "vue";
import type { Profile } from "@/types/auth";
import type { AppSettings, InstanceConfig } from "@/types/settings";
import type { LaunchOptions } from "@/types/launch";

interface LauncherDeps {
  profile: Ref<Profile | null>;
  instance: Ref<InstanceConfig | null>;
  settings: Ref<AppSettings>;
  setStatus: (message: string) => void;
  setProgress: (value: number) => void;
  run: <T>(task: () => Promise<T>) => Promise<T | undefined>;
  resolveGameDir: (instance: InstanceConfig | null) => string;
}

export function useLauncher({
  profile,
  instance,
  settings,
  setStatus,
  setProgress,
  run,
  resolveGameDir
}: LauncherDeps) {
  function buildOptions(): LaunchOptions | null {
    const active = instance.value;
    if (!active) {
      setStatus("Select a profile before launching.");
      return null;
    }
    const gameDir = resolveGameDir(active);
    if (!gameDir) {
      setStatus("Select a profile with a valid data folder.");
      return null;
    }
    const loader = active.loader ?? { kind: "vanilla", loaderVersion: null };
    if (loader.kind === "fabric" && !(active.version ?? "").trim()) {
      setStatus("Choose a Minecraft version for Fabric.");
      return null;
    }
    if (loader.kind === "neoforge" && !(loader.loaderVersion ?? "").trim()) {
      setStatus("Set a NeoForge loader version before launching.");
      return null;
    }
    const defaultMemory = Math.max(1024, settings.value.defaultMemoryMb ?? 4096);
    const memoryMb =
      typeof active.memoryMb === "number" && Number.isFinite(active.memoryMb)
        ? Math.max(1024, Math.round(active.memoryMb))
        : defaultMemory;
    const jvmArgs = (active.jvmArgs ?? "").trim() || (settings.value.defaultJvmArgs ?? "");

    return {
      gameDir,
      javaPath: active.javaPath ?? "",
      memoryMb,
      jvmArgs,
      version: active.version ?? null,
      loader
    };
  }

  async function launchMinecraft() {
    if (!profile.value) {
      setStatus("Sign in with Microsoft before launching.");
      return;
    }
    const options = buildOptions();
    if (!options) {
      return;
    }
    setProgress(0);
    setStatus("Starting Minecraft...");
    await run(async () => {
      try {
        await invoke("launch_minecraft", { options });
      } catch (err) {
        setStatus(`Could not start Minecraft: ${String(err)}`);
      }
    });
  }

  async function downloadMinecraftFiles() {
    const options = buildOptions();
    if (!options) {
      return;
    }
    setProgress(0);
    await run(async () => {
      try {
        await invoke("download_minecraft_files", { options });
        setStatus("Game files are ready.");
      } catch (err) {
        setStatus(`Could not prepare files: ${String(err)}`);
      }
    });
  }

  return { launchMinecraft, downloadMinecraftFiles };
}
