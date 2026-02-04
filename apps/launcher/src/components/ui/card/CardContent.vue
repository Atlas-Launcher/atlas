<script setup lang="ts">
import { computed, useAttrs } from "vue";
import { cn } from "../../../lib/utils";

const attrs = useAttrs();
const classes = computed(() => cn("p-6 pt-0", attrs.class));
</script>

<template>
  <div v-bind="attrs" :class="classes">
    <slot />
  </div>
</template>
