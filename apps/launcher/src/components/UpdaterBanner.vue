<script setup lang="ts">
import { computed } from "vue";
import Button from "./ui/button/Button.vue";
import Card from "./ui/card/Card.vue";
import CardContent from "./ui/card/CardContent.vue";
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
  (event: "install"): void;
  (event: "restart"): void;
}>();

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
    <Card class="glass">
      <CardContent class="px-5 py-4">
        <div class="flex items-center justify-between gap-3">
          <div class="min-w-0">
            <p class="text-sm font-semibold text-foreground">
              <template v-if="props.installComplete">
                Update ready
              </template>
              <template v-else>
                Launcher update available
              </template>
            </p>
            <p v-if="props.updateInfo" class="text-sm text-muted-foreground">
              <template v-if="props.installComplete">
                Restart Atlas Launcher to finish updating to {{ props.updateInfo.version }}.
              </template>
              <template v-else>
                Version {{ props.updateInfo.version }} is ready (current {{ props.updateInfo.currentVersion }}).
              </template>
            </p>
          </div>
          <div class="flex items-center gap-2 shrink-0">
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
        </div>
        <div v-if="props.installing" class="mt-3 w-full space-y-2">
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
    class="fixed inset-0 z-[80] flex items-center justify-center bg-black/55 backdrop-blur-[6px] px-4 pb-4 pt-14 md:px-6 md:pb-6 md:pt-16"
    @click.self="emit('close')"
  >
    <Card class="glass relative w-full max-w-4xl max-h-[85vh] overflow-y-auto">
      <CardContent class="space-y-5 px-6 py-6">
        <div class="space-y-1">
          <p class="text-lg font-semibold text-foreground">Launcher update available</p>
          <p v-if="props.updateInfo" class="text-sm text-muted-foreground">
            Atlas Launcher {{ props.updateInfo.currentVersion }} -> {{ props.updateInfo.version }}
          </p>
        </div>

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
