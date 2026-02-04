<script setup lang="ts">
import { computed } from "vue";
import { cn } from "../../../lib/utils";

const props = withDefaults(
  defineProps<{
    modelValue?: number;
    class?: string;
  }>(),
  {
    modelValue: 0
  }
);

const percent = computed(() => Math.min(100, Math.max(0, props.modelValue)));
</script>

<template>
  <div :class="cn('relative h-2 w-full overflow-hidden rounded-full bg-secondary', props.class)">
    <div
      class="h-full rounded-full bg-primary transition-all"
      :style="{ width: `${percent}%` }"
    ></div>
  </div>
</template>
