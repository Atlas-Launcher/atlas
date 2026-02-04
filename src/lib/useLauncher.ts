import { invoke } from "@tauri-apps/api/core";
import type { Ref } from "vue";
import type { Profile } from "@/types/auth";
import type { InstanceConfig } from "@/types/settings";
import type { LaunchOptions } from "@/types/launch";

interface LauncherDeps {
  profile: Ref<Profile | null>;
  instance: Ref<InstanceConfig | null>;
  setStatus: (message: string) => void;
  setProgress: (value: number) => void;
  run: <T>(task: () => Promise<T>) => Promise<T | undefined>;
}

export function useLauncher({ profile, instance, setStatus, setProgress, run }: LauncherDeps) {
  function buildOptions(): LaunchOptions | null {
    const active = instance.value;
    if (!active) {
      setStatus("Select an instance before launching.");
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
    return {
      gameDir: active.gameDir ?? "",
      javaPath: active.javaPath ?? "",
      memoryMb: active.memoryMb ?? 4096,
      version: active.version ?? null,
      loader
    };
  }

  async function launchMinecraft() {
    if (!profile.value) {
      setStatus("Sign in before launching.");
      return;
    }
    const options = buildOptions();
    if (!options) {
      return;
    }
    setProgress(0);
    await run(async () => {
      try {
        await invoke("launch_minecraft", { options });
        setStatus("Minecraft launched.");
      } catch (err) {
        setStatus(`Launch failed: ${String(err)}`);
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
        setStatus("Minecraft files downloaded.");
      } catch (err) {
        setStatus(`Download failed: ${String(err)}`);
      }
    });
  }

  return { launchMinecraft, downloadMinecraftFiles };
}
