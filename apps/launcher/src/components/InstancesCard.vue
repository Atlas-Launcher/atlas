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
import { Box } from "lucide-vue-next";
import type { InstanceConfig } from "@/types/settings";

const props = defineProps<{
  instances: InstanceConfig[];
  activeInstanceId: string | null;
  working: boolean;
}>();

const emit = defineEmits<{
  (event: "select", id: string): void;
  (event: "create"): void;
}>();

const search = ref("");
const filter = ref<"all" | "modded" | "vanilla" | "custom">("all");
const sort = ref<"name" | "loader">("name");
const group = ref<"none" | "loader">("none");

const filteredInstances = computed(() => {
  const query = search.value.trim().toLowerCase();
  let items = props.instances.filter((instance) => {
    if (filter.value === "modded" && instance.loader?.kind === "vanilla") {
      return false;
    }
    if (filter.value === "vanilla" && instance.loader?.kind !== "vanilla") {
      return false;
    }
    if (filter.value === "custom" && !(instance.gameDir ?? "").trim()) {
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
</script>

<template>
  <Card class="glass">
    <CardContent class="space-y-5">
      <div class="flex flex-wrap items-center pt-5 gap-3">
        <Tabs v-model="filter">
          <TabsList>
            <TabsTrigger value="all">All profiles</TabsTrigger>
            <TabsTrigger value="modded">Modded</TabsTrigger>
            <TabsTrigger value="vanilla">Vanilla</TabsTrigger>
            <TabsTrigger value="custom">Custom</TabsTrigger>
          </TabsList>
        </Tabs>
        <Button
          class="ml-auto"
          :disabled="props.working"
          size="sm"
          variant="secondary"
          @click="emit('create')"
        >
          New profile
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
          <button
            v-for="instance in grouping.items"
            :key="instance.id"
            class="flex items-center gap-3 rounded-2xl border border-border/60 bg-card/70 px-4 py-3 text-left transition hover:shadow-sm"
            :class="
              instance.id === props.activeInstanceId
                ? 'border-foreground/70 bg-foreground/5'
                : ''
            "
            @click="emit('select', instance.id)"
          >
            <div
              class="flex h-12 w-12 items-center justify-center rounded-xl border border-border/60 bg-muted"
            >
              <Box class="h-6 w-6 text-muted-foreground" />
            </div>
            <div class="flex-1">
              <div class="font-semibold text-foreground">{{ instance.name }}</div>
              <div class="text-xs text-muted-foreground">
                {{ instance.loader?.kind ?? "vanilla" }}
                ·
                {{ instance.version?.trim() ? instance.version : "Latest release" }}
              </div>
            </div>
          </button>
        </div>
      </div>
    </CardContent>
  </Card>
</template>
