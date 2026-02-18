import { invoke } from "@tauri-apps/api/core";

interface TakeWindowFocusOptions {
  label?: string;
  log?: (message: string) => void;
}

export async function takeWindowFocus(options: TakeWindowFocusOptions = {}) {
  const { label = "main", log } = options;

  try {
    await invoke("focus_window", { label });
    return true;
  } catch (err) {
    log?.(`Failed to focus ${label} window: ${String(err)}`);
    return false;
  }
}
