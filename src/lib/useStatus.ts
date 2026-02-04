import { ref } from "vue";

export function useStatus() {
  const status = ref("Ready");
  const logs = ref<string[]>([]);
  const progress = ref(0);

  function pushLog(entry: string) {
    logs.value = [entry, ...logs.value].slice(0, 8);
  }

  function setStatus(message: string) {
    status.value = message;
  }

  function setProgress(value: number) {
    progress.value = value;
  }

  return {
    status,
    logs,
    progress,
    pushLog,
    setStatus,
    setProgress
  };
}
