<script setup lang="ts">
import { computed } from "vue";
import Button from "./ui/button/Button.vue";
import Card from "./ui/card/Card.vue";
import CardContent from "./ui/card/CardContent.vue";
import CardDescription from "./ui/card/CardDescription.vue";
import CardFooter from "./ui/card/CardFooter.vue";
import CardHeader from "./ui/card/CardHeader.vue";
import CardTitle from "./ui/card/CardTitle.vue";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "./ui/select";
import type { InstanceConfig } from "@/types/settings";
import { formatLoaderKind } from "@/lib/utils";

const props = defineProps<{
  instance: InstanceConfig | null;
  working: boolean;
  installedVersions: string[];
}>();

const emit = defineEmits<{
  (event: "uninstall"): void;
  (event: "update-channel", value: "dev" | "beta" | "production"): void;
}>();

const hasInstalledFiles = computed(() => props.installedVersions.length > 0);
const selectedChannel = computed(() => props.instance?.atlasPack?.channel ?? "production");
const runtimeLabel = computed(() => {
  if (!props.instance) {
    return null;
  }
  const version = props.instance.version?.trim();
  if (!version) {
    return null;
  }
  return `${formatLoaderKind(props.instance.loader?.kind)} ${version}`;
});

function updateChannel(value: string) {
  if (value === "dev" || value === "beta" || value === "production") {
    emit("update-channel", value);
  }
}
</script>

<template>
  <Card class="glass">
    <CardHeader>
      <CardTitle>Manage remote pack</CardTitle>
      <CardDescription>
        Manage channel and local files for this Atlas Hub pack.
      </CardDescription>
    </CardHeader>
    <CardContent class="space-y-4">
      <div v-if="!props.instance" class="text-sm text-muted-foreground">
        Select a profile to manage.
      </div>

        <div v-else class="space-y-4">
          <div class="rounded-2xl border border-border/60 bg-card/70 px-4 py-3 text-sm">
            <div class="text-xs uppercase tracking-widest text-muted-foreground">Remote source</div>
            <div class="mt-1 font-semibold text-foreground">
              {{ props.instance.atlasPack?.packSlug ?? props.instance.name }}
            </div>
            <div class="mt-3 space-y-2">
              <div class="text-xs uppercase tracking-widest text-muted-foreground">Channel</div>
              <Select :disabled="props.working" :model-value="selectedChannel" @update:modelValue="updateChannel">
                <SelectTrigger class="h-8 max-w-[220px] text-xs">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent class="backdrop-blur-3xl bg-popover/95">
                  <SelectItem value="production">Production</SelectItem>
                  <SelectItem value="beta">Beta</SelectItem>
                  <SelectItem value="dev">Dev</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div v-if="props.instance.atlasPack?.buildVersion" class="mt-2 text-xs text-muted-foreground">
              Build {{ props.instance.atlasPack.buildVersion }}
            </div>
            <div v-if="runtimeLabel" class="mt-1 text-xs text-muted-foreground">
              Runtime {{ runtimeLabel }}
            </div>
          </div>

        <div v-if="hasInstalledFiles" class="flex flex-wrap gap-2">
          <Button :disabled="props.working" variant="destructive" @click="emit('uninstall')">
            Uninstall local files
          </Button>
        </div>
      </div>
    </CardContent>
    <CardFooter>
      <div class="text-xs text-muted-foreground">
        Version and loader are managed by Atlas Hub for remote packs.
      </div>
    </CardFooter>
  </Card>
</template>
