import { computed, ref } from "vue";

export function useWorking() {
  const activeCount = ref(0);
  const working = computed(() => activeCount.value > 0);

  async function run<T>(task: () => Promise<T>) {
    activeCount.value += 1;
    try {
      return await task();
    } finally {
      activeCount.value = Math.max(0, activeCount.value - 1);
    }
  }

  return { working, run };
}
