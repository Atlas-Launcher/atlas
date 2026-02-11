<script setup lang="ts">
import { computed, useAttrs } from "vue";
import type { ClassValue } from "clsx";
import { cn } from "../../../lib/utils";

const attrs = useAttrs();
const classes = computed(() => cn("p-6 pt-0", attrs.class as ClassValue));
</script>

<template>
  <div v-bind="attrs" :class="classes">
    <slot />
  </div>
</template>
