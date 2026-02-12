<script setup lang="ts">
import { computed, ref } from "vue";
import Button from "./ui/button/Button.vue";
import Card from "./ui/card/Card.vue";
import CardHeader from "./ui/card/CardHeader.vue";
import CardTitle from "./ui/card/CardTitle.vue";
import CardDescription from "./ui/card/CardDescription.vue";
import CardContent from "./ui/card/CardContent.vue";
import CardFooter from "./ui/card/CardFooter.vue";
import Input from "./ui/input/Input.vue";
import type { InstanceConfig } from "@/types/settings";
import type { ModEntry } from "@/types/library";

const props = defineProps<{
  instance: InstanceConfig | null;
  mods: ModEntry[];
  modsDir: string;
  working: boolean;
}>();

const emit = defineEmits<{
  (event: "toggle", payload: { fileName: string; enabled: boolean }): void;
  (event: "delete", fileName: string): void;
  (event: "refresh"): void;
  (event: "open-folder"): void;
}>();

const search = ref("");
const filteredMods = computed(() => {
  const query = search.value.trim().toLowerCase();
  if (!query) {
    return props.mods;
  }
  return props.mods.filter((mod) => {
    return (
      mod.displayName.toLowerCase().includes(query) ||
      mod.fileName.toLowerCase().includes(query)
    );
  });
});

function formatBytes(size: number) {
  if (!size && size !== 0) {
    return "";
  }
  if (size < 1024) {
    return `${size} B`;
  }
  const kb = size / 1024;
  if (kb < 1024) {
    return `${kb.toFixed(1)} KB`;
  }
  const mb = kb / 1024;
  return `${mb.toFixed(1)} MB`;
}

function formatDate(seconds: number) {
  if (!seconds) {
    return "Unknown";
  }
  return new Date(seconds * 1000).toLocaleString();
}
</script>

<template>
  <Card class="glass h-full min-h-0 rounded-2xl flex flex-col">
    <CardHeader class="pt-7">
      <CardTitle>Mods</CardTitle>
      <CardDescription>Manage mods for the active profile.</CardDescription>
    </CardHeader>
    <CardContent class="flex-1 min-h-0 overflow-y-auto space-y-4 pr-3 pb-5 pt-1 [scrollbar-gutter:stable]">
      <div v-if="!props.instance" class="text-sm text-muted-foreground">
        Select a profile to manage mods.
      </div>

      <div v-else class="space-y-4">
        <div class="grid gap-3 md:grid-cols-[1.4fr_0.6fr]">
          <Input
            :model-value="search"
            placeholder="Search mods"
            @update:modelValue="(value) => (search = String(value))"
          />
          <div class="flex flex-wrap items-center justify-end gap-2">
            <Button :disabled="props.working" size="sm" variant="secondary" @click="emit('open-folder')">
              Open folder
            </Button>
            <Button :disabled="props.working" size="sm" variant="secondary" @click="emit('refresh')">
              Refresh
            </Button>
          </div>
        </div>
        <div class="rounded-2xl border border-border/60 bg-card/70 px-4 py-3 text-xs text-muted-foreground">
          Mods are stored in <span class="font-semibold text-foreground mono">{{ props.modsDir }}</span>. Add `.jar` or `.zip`
          files to install them.
        </div>

        <div class="space-y-2">
          <div
            v-for="mod in filteredMods"
            :key="mod.fileName"
            class="flex flex-col gap-2 rounded-2xl border border-border/60 bg-card/70 px-4 py-3"
          >
            <div class="flex flex-wrap items-center justify-between gap-3">
              <div>
                <div class="font-semibold text-foreground">{{ mod.displayName }}</div>
                <div class="text-xs text-muted-foreground">
                  {{ mod.fileName }}
                </div>
              </div>
              <div class="flex flex-wrap items-center gap-2">
                <Button
                  :disabled="props.working"
                  size="sm"
                  :variant="mod.enabled ? 'secondary' : 'outline'"
                  @click="emit('toggle', { fileName: mod.fileName, enabled: !mod.enabled })"
                >
                  {{ mod.enabled ? "Disable" : "Enable" }}
                </Button>
                <Button
                  :disabled="props.working"
                  size="sm"
                  variant="destructive"
                  @click="emit('delete', mod.fileName)"
                >
                  Remove
                </Button>
              </div>
            </div>
            <div class="flex flex-wrap gap-4 text-xs text-muted-foreground">
              <span>Size: {{ formatBytes(mod.size) }}</span>
              <span>Updated: {{ formatDate(mod.modified) }}</span>
            </div>
          </div>
          <div v-if="filteredMods.length === 0" class="text-sm text-muted-foreground">
            No mods added yet.
          </div>
        </div>
      </div>
    </CardContent>
    <CardFooter class="pt-0">
      <div class="text-xs text-muted-foreground">
        Mods need a compatible loader for the selected profile.
      </div>
    </CardFooter>
  </Card>
</template>
