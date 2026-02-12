<script setup lang="ts">
import { computed } from "vue";
import { X } from "lucide-vue-next";
import Button from "./ui/button/Button.vue";
import Card from "./ui/card/Card.vue";
import CardContent from "./ui/card/CardContent.vue";
import CardDescription from "./ui/card/CardDescription.vue";
import CardHeader from "./ui/card/CardHeader.vue";
import CardTitle from "./ui/card/CardTitle.vue";
import Progress from "./ui/progress/Progress.vue";
import type { ReleaseInfo } from "@/lib/useUpdater";

const props = defineProps<{
  visible: boolean;
  open: boolean;
  checking: boolean;
  installing: boolean;
  installComplete: boolean;
  progressPercent: number;
  downloadedBytes: number;
  totalBytes: number | null;
  updateInfo: ReleaseInfo | null;
  errorMessage: string | null;
}>();

const emit = defineEmits<{
  (event: "open"): void;
  (event: "close"): void;
  (event: "dismiss"): void;
  (event: "install"): void;
  (event: "restart"): void;
}>();

const releaseDate = computed(() => {
  const value = props.updateInfo?.date;
  if (!value) {
    return null;
  }
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) {
    return value;
  }
  return parsed.toLocaleString();
});

function formatBytes(value: number) {
  if (!Number.isFinite(value) || value <= 0) {
    return "0 B";
  }
  const units = ["B", "KB", "MB", "GB"];
  let size = value;
  let idx = 0;
  while (size >= 1024 && idx < units.length - 1) {
    size /= 1024;
    idx += 1;
  }
  return `${size.toFixed(idx === 0 ? 0 : 1)} ${units[idx]}`;
}
</script>

<template>
  <div v-if="props.visible" class="pointer-events-auto">
    <Card class="glass border-primary/40 bg-background/85 shadow-[0_24px_50px_-30px_rgba(0,0,0,0.75)] backdrop-blur-xl">
      <CardHeader class="space-y-1 pb-2">
        <div class="flex items-start justify-between gap-3">
          <CardTitle class="text-sm">
          <template v-if="props.installComplete">
            Update installed
          </template>
          <template v-else>
            Launcher update available
          </template>
          </CardTitle>
          <Button
            size="icon"
            variant="ghost"
            class="h-7 w-7 text-muted-foreground hover:text-foreground"
            aria-label="Dismiss update banner"
            @click="emit('dismiss')"
          >
            <X class="h-4 w-4" />
          </Button>
        </div>
        <CardDescription v-if="props.updateInfo">
          <template v-if="props.installComplete">
            Restart Atlas Launcher to apply version {{ props.updateInfo.version }}.
          </template>
          <template v-else>
            Version {{ props.updateInfo.version }} is available (current {{ props.updateInfo.currentVersion }}).
          </template>
        </CardDescription>
      </CardHeader>
      <CardContent class="space-y-3 pt-0">
        <div class="flex flex-wrap items-center mt-2 gap-2">
          <Button
            size="sm"
            variant="outline"
            :disabled="props.checking || props.installing"
            @click="emit('open')"
          >
            Details
          </Button>
          <Button
            v-if="props.installComplete"
            size="sm"
            :disabled="props.checking || props.installing"
            @click="emit('restart')"
          >
            Restart now
          </Button>
          <Button
            v-else
            size="sm"
            :disabled="props.checking || props.installing"
            @click="emit('install')"
          >
            {{ props.installing ? "Installing..." : "Install update" }}
          </Button>
        </div>
        <div v-if="props.installing" class="w-full space-y-2 pt-1">
          <Progress :model-value="props.progressPercent" />
          <p class="text-xs text-muted-foreground">
            <template v-if="props.totalBytes">
              {{ formatBytes(props.downloadedBytes) }} / {{ formatBytes(props.totalBytes) }}
              ({{ props.progressPercent }}%)
            </template>
            <template v-else>
              Downloading update...
            </template>
          </p>
        </div>
      </CardContent>
    </Card>
  </div>

  <div
    v-if="props.open"
    class="fixed inset-0 z-[80] flex items-center justify-center bg-black/55 backdrop-blur-[6px] p-4"
    @click.self="emit('close')"
  >
    <Card class="glass w-full max-w-2xl max-h-[85vh] overflow-y-auto border-primary/40 bg-background/90">
      <CardHeader class="space-y-1 pb-2">
        <CardTitle>Launcher update</CardTitle>
        <CardDescription v-if="props.updateInfo">
          Atlas Launcher {{ props.updateInfo.currentVersion }} -> {{ props.updateInfo.version }}
        </CardDescription>
      </CardHeader>
      <CardContent class="space-y-5 pt-0">
        <p v-if="releaseDate" class="text-xs text-muted-foreground">Published: {{ releaseDate }}</p>
        <div
          v-if="props.updateInfo?.body"
          class="rounded-lg border border-border bg-muted/20 p-3 text-sm text-muted-foreground whitespace-pre-wrap"
        >
          {{ props.updateInfo.body }}
        </div>
        <p v-else class="text-sm text-muted-foreground">No release notes were provided.</p>

        <div v-if="props.installing" class="space-y-2">
          <Progress :model-value="props.progressPercent" />
          <p class="text-xs text-muted-foreground">
            <template v-if="props.totalBytes">
              {{ formatBytes(props.downloadedBytes) }} / {{ formatBytes(props.totalBytes) }}
              ({{ props.progressPercent }}%)
            </template>
            <template v-else>
              Downloading update package...
            </template>
          </p>
        </div>

        <p v-if="props.installComplete" class="text-sm text-primary">
          Update installed. Restart Atlas Launcher to finish.
        </p>
        <p v-if="props.errorMessage" class="text-sm text-destructive">
          {{ props.errorMessage }}
        </p>

        <div class="flex flex-wrap gap-2">
          <Button variant="outline" @click="emit('close')">Close</Button>
          <Button
            v-if="props.installComplete"
            :disabled="props.checking || props.installing"
            @click="emit('restart')"
          >
            Restart now
          </Button>
          <Button
            v-else
            :disabled="props.checking || props.installing"
            @click="emit('install')"
          >
            {{ props.installing ? "Installing..." : "Install update" }}
          </Button>
        </div>
      </CardContent>
    </Card>
  </div>
</template>
