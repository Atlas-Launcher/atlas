import { ref } from "vue";
import type { LaunchEvent } from "@/types/launch";

export interface ActiveTask {
  id: string;
  phase: string;
  message: string;
  percent: number;
  startedAt: number;
  lastUpdated: number;
}

export function useStatus() {
  const status = ref("Ready");
  const logs = ref<string[]>([]);
  const progress = ref(0);
  const tasks = ref<ActiveTask[]>([]);
  const ACTIVE_LAUNCH_TASK_ID = "launch:active";
  const ACTIVE_LAUNCH_TASK_STALE_MS = 300000;

  function pushLog(entry: string) {
    logs.value = [entry, ...logs.value].slice(0, 200);
  }

  function setStatus(message: string) {
    status.value = message;
  }

  function setProgress(value: number) {
    progress.value = value;
  }

  function startTask(label: string, id: string = crypto.randomUUID()) {
    const now = Date.now();
    tasks.value.push({
      id,
      phase: label,
      message: label,
      percent: 0,
      startedAt: now,
      lastUpdated: now
    });
    return id;
  }

  function updateTask(id: string, patch: Partial<ActiveTask>) {
    const now = Date.now();
    const task = tasks.value.find((item) => item.id === id);
    if (!task) {
      return;
    }
    Object.assign(task, patch, { lastUpdated: now });
  }

  function finishTask(id: string) {
    tasks.value = tasks.value.filter((task) => task.id !== id);
  }

  function failTask(id: string, message?: string) {
    const task = tasks.value.find((item) => item.id === id);
    if (task && message) {
      task.message = message;
    }
    tasks.value = tasks.value.filter((item) => item.id !== id);
  }

  async function runTask<T>(label: string, task: () => Promise<T>) {
    const id = startTask(label);
    try {
      const result = await task();
      finishTask(id);
      return result;
    } catch (err) {
      failTask(id, String(err));
      throw err;
    }
  }

  function upsertTaskFromEvent(event: LaunchEvent) {
    const now = Date.now();
    const id = ACTIVE_LAUNCH_TASK_ID;
    const percent =
      typeof event.percent === "number"
        ? Math.round(event.percent)
        : event.current && event.total
          ? Math.round((event.current / event.total) * 100)
          : 0;

    const existing = tasks.value.find((task) => task.id === id);
    if (existing) {
      existing.phase = event.phase;
      existing.message = event.message;
      if (percent > 0 || typeof event.percent === "number") {
        existing.percent = percent;
      }
      existing.lastUpdated = now;
    } else {
      tasks.value.push({
        id,
        phase: event.phase,
        message: event.message,
        percent,
        startedAt: now,
        lastUpdated: now
      });
    }

    const doneMessage = event.message.toLowerCase();
    const isLaunchPhase = event.phase.toLowerCase() === "launch";
    const isLaunchComplete = isLaunchPhase && percent >= 100;
    const isFailure = doneMessage.includes("failed") || doneMessage.includes("error");
    const isNonLaunchComplete =
      !isLaunchPhase && (doneMessage.includes("ready") || doneMessage.includes("complete"));

    if (isLaunchComplete || isFailure || isNonLaunchComplete) {
      tasks.value = tasks.value.filter((task) => task.id !== id);
    }

    const cutoff = now - ACTIVE_LAUNCH_TASK_STALE_MS;
    tasks.value = tasks.value.filter((task) => {
      if (task.id !== id) {
        return true;
      }
      return task.lastUpdated >= cutoff;
    });
  }

  return {
    status,
    logs,
    progress,
    tasks,
    pushLog,
    setStatus,
    setProgress,
    startTask,
    updateTask,
    finishTask,
    failTask,
    runTask,
    upsertTaskFromEvent
  };
}
