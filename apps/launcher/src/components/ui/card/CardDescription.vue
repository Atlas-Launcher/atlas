<script setup lang="ts">
import { computed, useAttrs } from "vue";
import type { ClassValue } from "clsx";
import { cn } from "../../../lib/utils";

const attrs = useAttrs();
const classes = computed(() => cn("text-sm text-muted-foreground", attrs.class as ClassValue));
</script>

<template>
  <p v-bind="attrs" :class="classes">
    <slot />
  </p>
</template>
