import { invoke } from "@tauri-apps/api/core";
import type { Ref } from "vue";
import type { Profile } from "@/types/auth";

interface LauncherDeps {
  profile: Ref<Profile | null>;
  setStatus: (message: string) => void;
  setProgress: (value: number) => void;
  run: <T>(task: () => Promise<T>) => Promise<T | undefined>;
}

export function useLauncher({ profile, setStatus, setProgress, run }: LauncherDeps) {
  async function launchMinecraft() {
    if (!profile.value) {
      setStatus("Sign in before launching.");
      return;
    }
    setProgress(0);
    await run(async () => {
      try {
        await invoke("launch_minecraft", { options: {} });
        setStatus("Minecraft launched.");
      } catch (err) {
        setStatus(`Launch failed: ${String(err)}`);
      }
    });
  }

  async function downloadMinecraftFiles() {
    setProgress(0);
    await run(async () => {
      try {
        await invoke("download_minecraft_files", { options: {} });
        setStatus("Minecraft files downloaded.");
      } catch (err) {
        setStatus(`Download failed: ${String(err)}`);
      }
    });
  }

  return { launchMinecraft, downloadMinecraftFiles };
}
