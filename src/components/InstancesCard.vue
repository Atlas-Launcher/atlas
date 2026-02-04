<script setup lang="ts">
import { computed } from "vue";
import Button from "./ui/button/Button.vue";
import Card from "./ui/card/Card.vue";
import CardHeader from "./ui/card/CardHeader.vue";
import CardTitle from "./ui/card/CardTitle.vue";
import CardDescription from "./ui/card/CardDescription.vue";
import CardContent from "./ui/card/CardContent.vue";
import Input from "./ui/input/Input.vue";
import type { InstanceConfig } from "@/types/settings";

const props = defineProps<{
  instances: InstanceConfig[];
  activeInstanceId: string | null;
  working: boolean;
}>();

const emit = defineEmits<{
  (event: "select", id: string): void;
  (event: "create"): void;
  (event: "duplicate", id: string): void;
  (event: "remove", id: string): void;
  (event: "update", payload: { id: string; patch: Partial<InstanceConfig> }): void;
}>();

const activeInstance = computed(
  () => props.instances.find((instance) => instance.id === props.activeInstanceId) ?? null
);

function updateField(key: keyof InstanceConfig, value: string | number) {
  if (!activeInstance.value) {
    return;
  }
  const patch: Partial<InstanceConfig> = { [key]: value } as Partial<InstanceConfig>;
  emit("update", { id: activeInstance.value.id, patch });
}
</script>

<template>
  <Card class="glass">
    <CardHeader>
      <CardTitle>Instances</CardTitle>
      <CardDescription>Run multiple Minecraft setups with isolated files.</CardDescription>
    </CardHeader>
    <CardContent class="grid gap-6 lg:grid-cols-[1fr_1.2fr]">
      <div class="space-y-3">
        <div class="flex items-center justify-between">
          <div class="text-xs uppercase tracking-widest text-muted-foreground">Profiles</div>
          <Button :disabled="props.working" size="sm" variant="secondary" @click="emit('create')">
            New instance
          </Button>
        </div>
        <div class="space-y-2">
          <button
            v-for="instance in props.instances"
            :key="instance.id"
            class="flex w-full flex-col gap-1 rounded-2xl border px-4 py-3 text-left transition"
            :class="
              instance.id === props.activeInstanceId
                ? 'border-primary/60 bg-primary/10 text-foreground'
                : 'border-border/40 bg-secondary/40 text-muted-foreground hover:text-foreground'
            "
            @click="emit('select', instance.id)"
          >
            <div class="flex items-center justify-between">
              <span class="font-semibold">{{ instance.name }}</span>
              <span class="text-xs uppercase tracking-widest">
                {{ instance.loader?.kind ?? "vanilla" }}
              </span>
            </div>
            <div class="text-xs">
              {{ instance.version?.trim() ? instance.version : "Latest release" }}
            </div>
          </button>
        </div>
      </div>

      <div v-if="activeInstance" class="space-y-4">
        <div class="text-xs uppercase tracking-widest text-muted-foreground">Details</div>
        <div class="space-y-2">
          <label class="text-xs uppercase tracking-widest text-muted-foreground">Name</label>
          <Input
            :model-value="activeInstance.name"
            @update:modelValue="(value) => updateField('name', String(value))"
          />
        </div>
        <div class="space-y-2">
          <label class="text-xs uppercase tracking-widest text-muted-foreground">Game directory</label>
          <Input
            :model-value="activeInstance.gameDir"
            placeholder="Leave blank for the default location"
            @update:modelValue="(value) => updateField('gameDir', String(value))"
          />
        </div>
        <div class="grid gap-3 md:grid-cols-2">
          <div class="space-y-2">
            <label class="text-xs uppercase tracking-widest text-muted-foreground">Memory (MB)</label>
            <Input
              type="number"
              min="1024"
              :model-value="activeInstance.memoryMb ?? 4096"
              @update:modelValue="(value) => updateField('memoryMb', Number(value))"
            />
          </div>
          <div class="space-y-2">
            <label class="text-xs uppercase tracking-widest text-muted-foreground">Java path</label>
            <Input
              :model-value="activeInstance.javaPath ?? ''"
              placeholder="Auto-managed when empty"
              @update:modelValue="(value) => updateField('javaPath', String(value))"
            />
          </div>
        </div>
        <div class="flex flex-wrap gap-2">
          <Button
            :disabled="props.working"
            size="sm"
            variant="secondary"
            @click="emit('duplicate', activeInstance.id)"
          >
            Duplicate
          </Button>
          <Button
            :disabled="props.working || props.instances.length <= 1"
            size="sm"
            variant="destructive"
            @click="emit('remove', activeInstance.id)"
          >
            Delete
          </Button>
        </div>
      </div>
    </CardContent>
  </Card>
</template>
