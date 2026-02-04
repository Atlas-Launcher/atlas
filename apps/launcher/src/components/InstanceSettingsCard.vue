<script setup lang="ts">
import Button from "./ui/button/Button.vue";
import Card from "./ui/card/Card.vue";
import CardHeader from "./ui/card/CardHeader.vue";
import CardTitle from "./ui/card/CardTitle.vue";
import CardContent from "./ui/card/CardContent.vue";
import Input from "./ui/input/Input.vue";
import {Label} from "@/components/ui/label";
import type { InstanceConfig } from "@/types/settings";
import {computed} from "vue";
import { pruneHomePath } from "@/lib/utils";

const props = defineProps<{
  instance: InstanceConfig | null;
  instancesCount: number;
  working: boolean;
}>();

const displayGameDir = computed(() => {
  if (!props.instance) return "";
  // Use the utility to show ~ for the user's home/profile on macOS/Linux and Windows
  return pruneHomePath(props.instance.gameDir) ?? "Managed by the launcher";
});

function copyGameDir() {
  const dir = props.instance?.gameDir;
  if (!dir || !navigator.clipboard) return;
  navigator.clipboard.writeText(dir).catch(() => {});
}

const emit = defineEmits<{
  (event: "duplicate", id: string): void;
  (event: "remove", id: string): void;
  (event: "update", payload: { id: string; patch: Partial<InstanceConfig> }): void;
}>();

function updateField(key: keyof InstanceConfig, value: string | number) {
  if (!props.instance) {
    return;
  }
  const patch: Partial<InstanceConfig> = { [key]: value } as Partial<InstanceConfig>;
  emit("update", { id: props.instance.id, patch });
}
</script>

<template>
  <Card class="glass">
    <CardHeader>
      <CardTitle>Profile settings</CardTitle>
    </CardHeader>
    <CardContent class="space-y-4">
      <div v-if="!props.instance" class="text-sm text-muted-foreground">
        Select a profile to edit its settings.
      </div>
      <div v-else class="space-y-4">
        <div class="space-y-2">
          <Label class="text-xs uppercase tracking-widest text-muted-foreground">Name</Label>
          <Input
            :model-value="props.instance.name"
            @update:modelValue="(value) => updateField('name', String(value))"
          />
        </div>
        <div class="space-y-2">
          <Label class="text-xs uppercase tracking-widest text-muted-foreground">
            Data directory
          </Label>
          <div class="flex items-center gap-2">
            <!-- Display-only element. Clicking (or pressing Enter/Space) copies the full original path. -->
            <div
              role="button"
              tabindex="0"
              class="flex-1 truncate cursor-pointer text-sm text-foreground/90"
              :title="props.instance?.gameDir ?? 'Managed by the launcher'"
              @click="copyGameDir"
              @keydown.enter.prevent="copyGameDir"
              @keydown.space.prevent="copyGameDir"
              aria-label="Data directory (click to copy)"
            >
              {{ displayGameDir }}
            </div>
          </div>
        </div>

        <details class="rounded-2xl border border-border/60 bg-card/70 px-4 py-3">
          <summary class="cursor-pointer text-sm font-semibold text-foreground">
            Advanced options
          </summary>
          <div class="mt-3 grid gap-3 md:grid-cols-2">
            <div class="space-y-2">
              <Label class="text-xs uppercase tracking-widest text-muted-foreground">
                Memory (MB)
              </Label>
              <Input
                type="number"
                min="1024"
                :model-value="props.instance.memoryMb ?? 4096"
                @update:modelValue="(value) => updateField('memoryMb', Number(value))"
              />
            </div>
            <div class="space-y-2">
              <Label class="text-xs uppercase tracking-widest text-muted-foreground">
                Java path
              </Label>
              <Input
                :model-value="props.instance.javaPath ?? ''"
                placeholder="Leave empty to auto-manage Java"
                @update:modelValue="(value) => updateField('javaPath', String(value))"
              />
            </div>
          </div>
        </details>

        <div class="flex flex-wrap gap-2">
          <Button
            :disabled="props.working"
            size="sm"
            variant="secondary"
            @click="emit('duplicate', props.instance.id)"
          >
            Duplicate profile
          </Button>
          <Button
            :disabled="props.working || props.instancesCount <= 1"
            size="sm"
            variant="destructive"
            @click="emit('remove', props.instance.id)"
          >
            Delete profile
          </Button>
        </div>
      </div>
    </CardContent>
  </Card>
</template>
