<script setup lang="ts">
import InstancesCard from "./InstancesCard.vue";
import type { InstanceConfig } from "@/types/settings";

const props = defineProps<{
  instances: InstanceConfig[];
  activeInstanceId: string | null;
  working: boolean;
}>();

const emit = defineEmits<{
  (event: "select", id: string): void;
  (event: "create"): void;
  (event: "refresh-packs"): void;
}>();
</script>

<template>
  <InstancesCard
    :instances="props.instances"
    :active-instance-id="props.activeInstanceId"
    :working="props.working"
    @select="emit('select', $event)"
    @create="emit('create')"
    @refresh-packs="emit('refresh-packs')"
  />
</template>
