import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { AppSettings } from "@/types/settings";

interface SettingsDeps {
  setStatus: (message: string) => void;
  pushLog: (entry: string) => void;
  run: <T>(task: () => Promise<T>) => Promise<T | undefined>;
}

export function useSettings({ setStatus, pushLog, run }: SettingsDeps) {
  const settingsClientId = ref("");

  async function loadSettings() {
    try {
      const settings = await invoke<AppSettings>("get_settings");
      settingsClientId.value = settings.msClientId ?? "";
    } catch (err) {
      pushLog(`Failed to load settings: ${String(err)}`);
    }
  }

  async function saveSettings() {
    await run(async () => {
      try {
        const trimmed = settingsClientId.value.trim();
        await invoke("update_settings", {
          settings: {
            msClientId: trimmed.length > 0 ? trimmed : null
          }
        });
        setStatus("Settings saved.");
      } catch (err) {
        setStatus(`Settings save failed: ${String(err)}`);
      }
    });
  }

  return {
    settingsClientId,
    loadSettings,
    saveSettings
  };
}
