import { listen } from "@tauri-apps/api/event";
import type { Ref } from "vue";
import type { LaunchEvent } from "@/types/launch";

interface LaunchEventsDeps {
  status: Ref<string>;
  progress: Ref<number>;
  pushLog: (entry: string) => void;
}

export async function initLaunchEvents({ status, progress, pushLog }: LaunchEventsDeps) {
  await listen<LaunchEvent>("launch://status", (event) => {
    const payload = event.payload;
    status.value = payload.message;
    if (typeof payload.percent === "number") {
      progress.value = payload.percent;
    } else if (payload.total && payload.current) {
      progress.value = Math.round((payload.current / payload.total) * 100);
    }
    pushLog(`${payload.phase}: ${payload.message}`);
  });
}
