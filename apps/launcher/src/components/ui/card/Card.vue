<script setup lang="ts">
import { computed, useAttrs } from "vue";
import type { ClassValue } from "clsx";
import { cn } from "../../../lib/utils";

const attrs = useAttrs();
const classes = computed(() => cn("rounded-xl border border-border bg-card text-card-foreground", attrs.class as ClassValue));
</script>

<template>
  <div v-bind="attrs" :class="classes">
    <slot />
  </div>
</template>
