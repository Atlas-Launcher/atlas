<script setup lang="ts">
import { computed, ref } from "vue";
import Button from "./ui/button/Button.vue";
import Card from "./ui/card/Card.vue";
import CardHeader from "./ui/card/CardHeader.vue";
import CardTitle from "./ui/card/CardTitle.vue";
import CardDescription from "./ui/card/CardDescription.vue";
import CardContent from "./ui/card/CardContent.vue";
import Input from "./ui/input/Input.vue";
import { Box } from "lucide-vue-next";
import type { InstanceConfig } from "@/types/settings";
import { formatLoaderKind } from "@/lib/utils";

const props = defineProps<{
  instances: InstanceConfig[];
  activeInstanceId: string | null;
  instanceInstallStateById: Record<string, boolean>;
  working: boolean;
  canLaunch: boolean;
}>();

const emit = defineEmits<{
  (event: "select", id: string): void;
  (event: "play", id: string): void;
  (event: "install", id: string): void;
  (event: "create"): void;
  (event: "refresh-packs"): void;
}>();

const search = ref("");

const filteredInstances = computed(() => {
  const query = search.value.trim().toLowerCase();
  return [...props.instances]
    .filter((instance) => {
      if (!query) {
        return true;
      }
      return (
        instance.name.toLowerCase().includes(query) ||
        (instance.version ?? "").toLowerCase().includes(query)
      );
    })
    .sort((a, b) => a.name.localeCompare(b.name));
});

function displayLoader(instance: InstanceConfig) {
  const kind = instance.loader?.kind ?? "vanilla";
  if (!props.instanceInstallStateById[instance.id]) {
    return "Not installed";
  }
  return formatLoaderKind(kind);
}

function displayVersion(instance: InstanceConfig) {
  if (!props.instanceInstallStateById[instance.id]) {
    return null;
  }
  return instance.version?.trim() ? instance.version : "Latest release";
}

function onCardKeydown(event: KeyboardEvent, id: string) {
  if (event.key !== "Enter" && event.key !== " ") {
    return;
  }
  event.preventDefault();
  emit("select", id);
}

function needsRemoteInstall(instance: InstanceConfig) {
  return instance.source === "atlas" && !props.instanceInstallStateById[instance.id];
}

function actionLabel(instance: InstanceConfig) {
  return needsRemoteInstall(instance) ? "Install" : "Play";
}

function onPrimaryAction(instance: InstanceConfig) {
  if (needsRemoteInstall(instance)) {
    emit("install", instance.id);
    return;
  }
  if (!props.canLaunch) {
    return;
  }
  emit("play", instance.id);
}
</script>

<template>
  <Card class="glass h-full min-h-0 rounded-2xl flex flex-col">
    <CardHeader class="pt-7">
      <CardTitle>Your profiles</CardTitle>
      <CardDescription>Pick a profile, install if needed, then play.</CardDescription>
    </CardHeader>
    <CardContent class="flex-1 min-h-0 flex flex-col space-y-6 pr-3 pb-5 pt-1">
      <div class="grid gap-3 md:grid-cols-[1fr_auto]">
        <Input
          :model-value="search"
          placeholder="Search profiles"
          @update:modelValue="(value) => (search = String(value))"
        />
        <Button :disabled="props.working" size="sm" variant="secondary" @click="emit('refresh-packs')">
          Refresh
        </Button>
      </div>

      <div class="min-h-0 flex-1 overflow-y-auto snap-y snap-proximity pt-2 pb-2 pr-1 [scrollbar-gutter:stable]">
        <div v-if="filteredInstances.length === 0" class="text-sm text-muted-foreground px-1">
          No profiles available yet. Sync packs or create a local profile.
        </div>

        <div v-else class="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
          <div
            v-for="instance in filteredInstances"
            :key="instance.id"
            class="glass group rounded-2xl p-4 snap-start"
            :class="instance.id === props.activeInstanceId ? 'border-foreground/70 bg-foreground/5' : ''"
          >
            <button
              class="flex w-full cursor-pointer items-center gap-3 text-left"
              type="button"
              @click="emit('select', instance.id)"
              @keydown="(event) => onCardKeydown(event, instance.id)"
            >
              <div class="flex h-12 w-12 items-center justify-center rounded-xl border border-border/60 bg-muted">
                <Box class="h-6 w-6 text-muted-foreground" />
              </div>
              <div class="flex-1">
                <div class="font-semibold text-foreground">{{ instance.name }}</div>
                <div class="text-xs text-muted-foreground">
                  {{ displayLoader(instance) }} {{ displayVersion(instance) ? ` · ${displayVersion(instance)}` : "" }}
                </div>
              </div>
            </button>

            <div class="mt-3 flex items-center justify-end">
              <Button
                size="sm"
                :disabled="props.working || (!needsRemoteInstall(instance) && !props.canLaunch)"
                @click="onPrimaryAction(instance)"
              >
                {{ actionLabel(instance) }}
              </Button>
            </div>
          </div>
        </div>
      </div>
    </CardContent>
  </Card>
</template>
