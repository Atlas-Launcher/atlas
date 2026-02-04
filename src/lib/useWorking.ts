import { ref } from "vue";

export function useWorking() {
  const working = ref(false);

  async function run<T>(task: () => Promise<T>) {
    if (working.value) {
      return;
    }
    working.value = true;
    try {
      return await task();
    } finally {
      working.value = false;
    }
  }

  return { working, run };
}
