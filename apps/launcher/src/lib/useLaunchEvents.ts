import { listen } from "@tauri-apps/api/event";
import type { Ref } from "vue";
import type { LaunchEvent, LaunchLogEvent } from "@/types/launch";

interface LaunchEventsDeps {
  status: Ref<string>;
  progress: Ref<number>;
  pushLog: (entry: string) => void;
  upsertTaskFromEvent?: (event: LaunchEvent) => void;
}

export async function initLaunchEvents({
  status,
  progress,
  pushLog,
  upsertTaskFromEvent
}: LaunchEventsDeps) {
  await listen<LaunchEvent>("launch://status", (event) => {
    const payload = event.payload;
    status.value = payload.message;
    upsertTaskFromEvent?.(payload);
    if (typeof payload.percent === "number") {
      progress.value = payload.percent;
    } else if (payload.total && payload.current) {
      progress.value = Math.round((payload.current / payload.total) * 100);
    }
    pushLog(payload.message);
  });

  await listen<LaunchLogEvent>("launch://log", (event) => {
    const payload = event.payload;
    const prefix = payload.stream?.trim() ? `[mc:${payload.stream}] ` : "[mc] ";
    pushLog(`${prefix}${payload.message}`);
  });
}
