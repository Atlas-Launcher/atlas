<script setup lang="ts">
import Button from "./ui/button/Button.vue";
import Card from "./ui/card/Card.vue";
import CardHeader from "./ui/card/CardHeader.vue";
import CardTitle from "./ui/card/CardTitle.vue";
import CardDescription from "./ui/card/CardDescription.vue";
import CardContent from "./ui/card/CardContent.vue";
import CardFooter from "./ui/card/CardFooter.vue";
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
  <Card class="glass">
    <CardHeader>
      <CardTitle>Mods</CardTitle>
      <CardDescription>Toggle or remove mods for the active instance.</CardDescription>
    </CardHeader>
    <CardContent class="space-y-4">
      <div v-if="!props.instance" class="text-sm text-muted-foreground">
        Select an instance to manage mods.
      </div>

      <div v-else class="space-y-4">
        <div class="rounded-2xl border border-border/40 bg-secondary/40 px-4 py-3 text-xs">
          <div class="flex flex-wrap items-center justify-between gap-2">
            <div>
              <div class="uppercase tracking-widest text-muted-foreground">Mods folder</div>
              <div class="mt-1 font-semibold text-foreground mono">{{ props.modsDir }}</div>
            </div>
            <div class="flex gap-2">
              <Button :disabled="props.working" size="sm" variant="secondary" @click="emit('open-folder')">
                Open folder
              </Button>
              <Button :disabled="props.working" size="sm" variant="secondary" @click="emit('refresh')">
                Refresh
              </Button>
            </div>
          </div>
          <div class="mt-2 text-muted-foreground">
            Drop `.jar` or `.zip` files into the folder. Disabled mods end with `.disabled`.
          </div>
        </div>

        <div class="space-y-2">
          <div
            v-for="mod in props.mods"
            :key="mod.fileName"
            class="flex flex-col gap-2 rounded-2xl border border-border/40 bg-secondary/40 px-4 py-3"
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
          <div v-if="props.mods.length === 0" class="text-sm text-muted-foreground">
            No mods yet.
          </div>
        </div>
      </div>
    </CardContent>
    <CardFooter>
      <div class="text-xs text-muted-foreground">
        Mods require a supported loader for the selected instance.
      </div>
    </CardFooter>
  </Card>
</template>
