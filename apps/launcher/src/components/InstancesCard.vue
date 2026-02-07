<script setup lang="ts">
import { computed, ref } from "vue";
import Button from "./ui/button/Button.vue";
import Card from "./ui/card/Card.vue";
import CardHeader from "./ui/card/CardHeader.vue";
import CardTitle from "./ui/card/CardTitle.vue";
import CardDescription from "./ui/card/CardDescription.vue";
import CardContent from "./ui/card/CardContent.vue";
import Input from "./ui/input/Input.vue";
import { Tabs, TabsList, TabsTrigger } from "./ui/tabs";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue
} from "./ui/select";
import { Box, Download, Play } from "lucide-vue-next";
import type { InstanceConfig } from "@/types/settings";
import { formatLoaderKind } from "@/lib/utils";

const props = defineProps<{
  instances: InstanceConfig[];
  activeInstanceId: string | null;
  instanceInstallStateById: Record<string, boolean>;
  working: boolean;
}>();

const emit = defineEmits<{
  (event: "select", id: string): void;
  (event: "play", id: string): void;
  (event: "install", id: string): void;
  (event: "create"): void;
  (event: "refresh-packs"): void;
}>();

const search = ref("");
const filter = ref<"all" | "atlas" | "local">("all");
const sort = ref<"name" | "loader">("name");
const group = ref<"none" | "loader">("none");

const filteredInstances = computed(() => {
  const query = search.value.trim().toLowerCase();
  let items = props.instances.filter((instance) => {
    if (filter.value === "atlas" && instance.source !== "atlas") {
      return false;
    }
    if (filter.value === "local" && instance.source === "atlas") {
      return false;
    }
    if (!query) {
      return true;
    }
    return (
      instance.name.toLowerCase().includes(query) ||
      (instance.version ?? "").toLowerCase().includes(query)
    );
  });

  items = [...items].sort((a, b) => {
    if (sort.value === "loader") {
      const aLoader = a.loader?.kind ?? "vanilla";
      const bLoader = b.loader?.kind ?? "vanilla";
      if (aLoader !== bLoader) {
        return aLoader.localeCompare(bLoader);
      }
    }
    return a.name.localeCompare(b.name);
  });

  return items;
});

const groupedInstances = computed(() => {
  if (group.value !== "loader") {
    return [{ label: "All profiles", items: filteredInstances.value }];
  }
  const buckets: Record<string, InstanceConfig[]> = {};
  for (const instance of filteredInstances.value) {
    const key = instance.loader?.kind ?? "vanilla";
    if (!buckets[key]) {
      buckets[key] = [];
    }
    buckets[key].push(instance);
  }
  return Object.entries(buckets).map(([label, items]) => ({
    label,
    items
  }));
});

function displaySource(instance: InstanceConfig) {
  return instance.source === "atlas" ? "Atlas Hub" : "Local";
}

function displayLoader(instance: InstanceConfig) {
  const kind = instance.loader?.kind ?? "vanilla";
  if (props.instanceInstallStateById[instance.id] === false) {
    return "Not installed";
  }
  return formatLoaderKind(kind);
}

function displayVersion(instance: InstanceConfig) {
  if (props.instanceInstallStateById[instance.id] === false) {
    return null
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
  return instance.source === "atlas" && props.instanceInstallStateById[instance.id] === false;
}

function quickActionLabel(instance: InstanceConfig) {
  return needsRemoteInstall(instance) ? `Install ${instance.name}` : `Play ${instance.name}`;
}

function onQuickAction(instance: InstanceConfig) {
  if (needsRemoteInstall(instance)) {
    emit("install", instance.id);
    return;
  }
  emit("play", instance.id);
}
</script>

<template>
  <Card class="glass rounded-2xl">
    <CardContent class="space-y-6 pt-6">
      <div class="flex flex-wrap items-center gap-4">
        <Tabs v-model="filter">
          <TabsList>
            <TabsTrigger value="all">All Profiles</TabsTrigger>
            <TabsTrigger value="atlas">Atlas Profiles</TabsTrigger>
            <TabsTrigger value="local">Local Profiles</TabsTrigger>
          </TabsList>
        </Tabs>
        <Button
          :disabled="props.working"
          size="sm"
          variant="secondary"
          @click="emit('refresh-packs')"
        >
          Refresh Packs
        </Button>
        <Button
          class="ml-auto"
          :disabled="props.working"
          size="sm"
          variant="secondary"
          @click="emit('create')"
        >
          New Local Profile
        </Button>
      </div>

      <div class="grid gap-3 md:grid-cols-[1.3fr_0.6fr_0.6fr]">
        <Input
          :model-value="search"
          placeholder="Search profiles..."
          @update:modelValue="(value) => (search = String(value))"
        />
        <Select v-model="sort">
          <SelectTrigger>
            <SelectValue placeholder="Sort by" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="name">Sort by: Name</SelectItem>
            <SelectItem value="loader">Sort by: Loader</SelectItem>
          </SelectContent>
        </Select>
        <Select v-model="group">
          <SelectTrigger>
            <SelectValue placeholder="Group by" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="none">Group by: None</SelectItem>
            <SelectItem value="loader">Group by: Loader</SelectItem>
          </SelectContent>
        </Select>
      </div>

      <div v-if="groupedInstances.length === 0" class="text-sm text-muted-foreground">
        No profiles match the current filter.
      </div>

      <div v-for="grouping in groupedInstances" :key="grouping.label" class="space-y-3">
        <div
          v-if="groupedInstances.length > 1"
          class="text-xs uppercase tracking-widest text-muted-foreground"
        >
          {{ grouping.label }}
        </div>
        <div class="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
          <div
            v-for="instance in grouping.items"
            :key="instance.id"
            class="group flex cursor-pointer items-center gap-3 rounded-2xl border border-border/60 bg-card/70 px-4 py-3 text-left transition hover:shadow-sm"
            :class="
              instance.id === props.activeInstanceId
                ? 'border-foreground/70 bg-foreground/5'
                : ''
            "
            role="button"
            tabindex="0"
            @click="emit('select', instance.id)"
            @keydown="(event) => onCardKeydown(event, instance.id)"
          >
            <div
              class="relative flex h-12 w-12 items-center justify-center rounded-xl border border-border/60 bg-muted"
            >
              <Box class="h-6 w-6 text-muted-foreground transition-opacity group-hover:opacity-0 group-focus-within:opacity-0" />
              <button
                type="button"
                class="absolute inset-1 inline-flex items-center justify-center rounded-lg bg-primary text-primary-foreground opacity-0 transition-opacity hover:bg-primary/90 focus-visible:opacity-100 group-hover:opacity-100 group-focus-within:opacity-100"
                :aria-label="quickActionLabel(instance)"
                @click.stop="onQuickAction(instance)"
              >
                <Download v-if="needsRemoteInstall(instance)" class="h-4 w-4" />
                <Play v-else class="h-4 w-4" />
              </button>
            </div>
            <div class="flex-1">
              <div class="font-semibold text-foreground">{{ instance.name }}</div>
              <div class="text-xs text-muted-foreground">
                {{ displaySource(instance) }} · {{ displayLoader(instance) }} {{(displayVersion(instance)) ? ` · ${displayVersion(instance)}` : `` }}
              </div>
            </div>
          </div>
        </div>
      </div>
    </CardContent>
  </Card>
</template>
