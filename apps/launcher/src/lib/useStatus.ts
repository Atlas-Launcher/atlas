import { ref } from "vue";
import type { LaunchEvent } from "@/types/launch";

export interface ActiveTask {
  id: string;
  phase: string;
  message: string;
  percent: number;
  stageLabel: string;
  statusText: string;
  etaSeconds: number | null;
  indeterminate: boolean;
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
  const latestLaunchSuccessAt = ref<number | null>(null);
  let lastEtaSample:
    | {
      stageLabel: string;
      percent: number;
      at: number;
    }
    | null = null;

  function normalizeMessage(message: string | null | undefined) {
    const value = (message ?? "").trim();
    return value || "Working...";
  }

  function mapLaunchStage(event: LaunchEvent) {
    const phase = event.phase.trim().toLowerCase();
    if (phase === "atlas-sync") {
      return "Syncing pack";
    }
    if (
      phase === "setup" ||
      phase === "download" ||
      phase === "client" ||
      phase === "libraries" ||
      phase === "natives" ||
      phase === "assets"
    ) {
      return "Preparing files";
    }
    if (phase === "launch") {
      return "Starting Minecraft";
    }
    return "Preparing files";
  }

  function resolvePercent(event: LaunchEvent) {
    if (typeof event.percent === "number" && Number.isFinite(event.percent)) {
      return Math.max(0, Math.min(100, Math.round(event.percent)));
    }
    if (
      typeof event.current === "number" &&
      Number.isFinite(event.current) &&
      typeof event.total === "number" &&
      Number.isFinite(event.total) &&
      event.total > 0
    ) {
      return Math.max(0, Math.min(100, Math.round((event.current / event.total) * 100)));
    }
    return 0;
  }

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
      stageLabel: label,
      statusText: label,
      etaSeconds: null,
      indeterminate: true,
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
    const percent = resolvePercent(event);
    const stageLabel = mapLaunchStage(event);
    const statusText = normalizeMessage(event.message);
    const hasKnownProgress =
      typeof event.percent === "number" ||
      (typeof event.current === "number" && typeof event.total === "number" && event.total > 0);
    let etaSeconds: number | null = null;

    if (hasKnownProgress && percent > 0 && percent < 100) {
      if (
        lastEtaSample &&
        lastEtaSample.stageLabel === stageLabel &&
        percent > lastEtaSample.percent
      ) {
        const elapsedSeconds = (now - lastEtaSample.at) / 1000;
        const progressDelta = percent - lastEtaSample.percent;
        const progressRate = progressDelta / Math.max(elapsedSeconds, 0.1);
        if (elapsedSeconds >= 2 && progressDelta >= 2 && progressRate > 0) {
          const estimate = Math.round((100 - percent) / progressRate);
          if (Number.isFinite(estimate) && estimate > 0 && estimate < 7200) {
            etaSeconds = estimate;
          }
        }
      }
      lastEtaSample = {
        stageLabel,
        percent,
        at: now
      };
    } else {
      lastEtaSample = null;
    }

    const existing = tasks.value.find((task) => task.id === id);
    if (existing) {
      existing.phase = event.phase;
      existing.message = statusText;
      existing.stageLabel = stageLabel;
      existing.statusText = statusText;
      existing.indeterminate = !hasKnownProgress;
      existing.etaSeconds = etaSeconds;
      if (percent > 0 || typeof event.percent === "number") {
        existing.percent = percent;
      }
      existing.lastUpdated = now;
    } else {
      tasks.value.push({
        id,
        phase: event.phase,
        message: statusText,
        percent,
        stageLabel,
        statusText,
        etaSeconds,
        indeterminate: !hasKnownProgress,
        startedAt: now,
        lastUpdated: now
      });
    }

    const doneMessage = statusText.toLowerCase();
    const isLaunchPhase = event.phase.toLowerCase() === "launch";
    const isLaunchSuccess = isLaunchPhase && doneMessage.includes("window is on-screen");
    const isLaunchComplete = isLaunchPhase && percent >= 100;
    const isFailure = doneMessage.includes("failed") || doneMessage.includes("error");
    const isNonLaunchComplete =
      !isLaunchPhase && (doneMessage.includes("ready") || doneMessage.includes("complete"));

    if (isLaunchSuccess) {
      latestLaunchSuccessAt.value = now;
    }

    if (isLaunchComplete || isFailure || isNonLaunchComplete) {
      tasks.value = tasks.value.filter((task) => task.id !== id);
      lastEtaSample = null;
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
    upsertTaskFromEvent,
    latestLaunchSuccessAt
  };
}
