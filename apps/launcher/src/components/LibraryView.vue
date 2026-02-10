<script setup lang="ts">
import InstancesCard from "./InstancesCard.vue";
import type { InstanceConfig } from "@/types/settings";

const props = defineProps<{
  instances: InstanceConfig[];
  activeInstanceId: string | null;
  instanceInstallStateById: Record<string, boolean>;
  working: boolean;
  canLaunch: boolean;
  statusMessage?: string | null;
}>();

const emit = defineEmits<{
  (event: "select", id: string): void;
  (event: "play", id: string): void;
  (event: "install", id: string): void;
  (event: "create"): void;
  (event: "refresh-packs"): void;
}>();
</script>

<template>
  <div class="space-y-4">
    <div
      v-if="props.statusMessage"
      class="rounded-2xl border border-border/60 bg-card/70 px-4 py-3 text-sm text-muted-foreground"
    >
      {{ props.statusMessage }}
    </div>
    <InstancesCard
      :instances="props.instances"
      :active-instance-id="props.activeInstanceId"
      :instance-install-state-by-id="props.instanceInstallStateById"
      :working="props.working"
      :can-launch="props.canLaunch"
      @select="emit('select', $event)"
      @play="emit('play', $event)"
      @install="emit('install', $event)"
      @create="emit('create')"
      @refresh-packs="emit('refresh-packs')"
    />
  </div>
</template>
