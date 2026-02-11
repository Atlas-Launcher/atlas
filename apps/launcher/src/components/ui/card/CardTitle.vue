<script setup lang="ts">
import { computed, useAttrs } from "vue";
import type { ClassValue } from "clsx";
import { cn } from "../../../lib/utils";

const attrs = useAttrs();
const classes = computed(() => cn("text-xl font-semibold leading-none tracking-tight", attrs.class as ClassValue));
</script>

<template>
  <h3 v-bind="attrs" :class="classes">
    <slot />
  </h3>
</template>
