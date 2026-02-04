<script setup lang="ts">
import { computed, useAttrs } from "vue";
import { cn } from "../../../lib/utils";

const props = defineProps<{
  modelValue?: string | number;
}>();

const emit = defineEmits<{
  (event: "update:modelValue", value: string | number): void;
}>();

const attrs = useAttrs();
const classes = computed(() =>
  cn(
    "flex h-10 w-full rounded-md border border-input bg-transparent px-3 py-2 text-sm text-foreground placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50",
    attrs.class
  )
);

function onInput(event: Event) {
  const target = event.target as HTMLInputElement;
  if (target.type === "number") {
    emit("update:modelValue", target.value === "" ? "" : target.valueAsNumber);
    return;
  }
  emit("update:modelValue", target.value);
}
</script>

<template>
  <input v-bind="attrs" :class="classes" :value="modelValue" @input="onInput" />
</template>
