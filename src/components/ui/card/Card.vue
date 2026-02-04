<script setup lang="ts">
import { computed, useAttrs } from "vue";
import { cn } from "../../../lib/utils";

const attrs = useAttrs();
const classes = computed(() => cn("rounded-xl border border-border bg-card text-card-foreground shadow-sm", attrs.class));
</script>

<template>
  <div v-bind="attrs" :class="classes">
    <slot />
  </div>
</template>
