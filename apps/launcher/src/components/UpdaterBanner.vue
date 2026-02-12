<script setup lang="ts">
import { computed } from "vue";
import Button from "./ui/button/Button.vue";
import Card from "./ui/card/Card.vue";
import CardContent from "./ui/card/CardContent.vue";
import Progress from "./ui/progress/Progress.vue";
import type { ReleaseInfo } from "@/lib/useUpdater";

const props = defineProps<{
  visible: boolean;
  checking: boolean;
  installing: boolean;
  installComplete: boolean;
  progressPercent: number;
  downloadedBytes: number;
  totalBytes: number | null;
  speedBytesPerSecond: number | null;
  etaSeconds: number | null;
  updateInfo: ReleaseInfo | null;
  errorMessage: string | null;
}>();

const emit = defineEmits<{
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

function formatEta(seconds: number | null) {
  if (seconds == null || !Number.isFinite(seconds)) {
    return null;
  }
  const clamped = Math.max(0, Math.round(seconds));
  if (clamped < 60) {
    return `${clamped}s left`;
  }
  const minutes = Math.floor(clamped / 60);
  const remainder = clamped % 60;
  if (minutes < 60) {
    return `${minutes}m ${remainder}s left`;
  }
  const hours = Math.floor(minutes / 60);
  const minutesRemainder = minutes % 60;
  return `${hours}h ${minutesRemainder}m left`;
}

const speedLabel = computed(() => {
  if (!props.installing) {
    return null;
  }
  if (!props.speedBytesPerSecond || props.speedBytesPerSecond <= 0) {
    return "Calculating speed...";
  }
  return `${formatBytes(props.speedBytesPerSecond)}/s`;
});

const amountLabel = computed(() => {
  if (!props.installing) {
    return null;
  }
  if (props.totalBytes) {
    return `${formatBytes(props.downloadedBytes)} / ${formatBytes(props.totalBytes)} (${props.progressPercent}%)`;
  }
  return `${formatBytes(props.downloadedBytes)} downloaded`;
});

const etaLabel = computed(() => {
  if (!props.installing) {
    return null;
  }
  return formatEta(props.etaSeconds);
});
</script>

<template>
  <div v-if="props.visible" class="pointer-events-auto">
    <Card class="glass">
      <CardContent class="px-5 py-4">
        <div class="flex items-center justify-between gap-3">
          <div class="min-w-0">
            <p class="text-sm font-semibold text-foreground">
              <template v-if="props.installComplete">
                Apply update
              </template>
              <template v-else>
                Update available
              </template>
            </p>
            <p v-if="props.updateInfo" class="text-sm text-muted-foreground">
              <template v-if="props.installComplete">
                Version {{ props.updateInfo.version }} is ready. Relaunch to apply it.
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
              Relaunch
            </Button>
            <Button
              v-else
              size="sm"
              :disabled="props.checking || props.installing"
              @click="emit('install')"
            >
              {{ props.installing ? "Installing..." : "Install" }}
            </Button>
          </div>
        </div>
        <div v-if="props.installing" class="mt-3 w-full space-y-2">
          <Progress :model-value="props.progressPercent" />
          <p v-if="amountLabel" class="text-xs text-muted-foreground">{{ amountLabel }}</p>
          <p v-if="speedLabel || etaLabel" class="text-xs text-muted-foreground">
            <span v-if="speedLabel">{{ speedLabel }}</span>
            <span v-if="speedLabel && etaLabel"> • </span>
            <span v-if="etaLabel">{{ etaLabel }}</span>
          </p>
        </div>
        <p v-if="props.errorMessage" class="mt-2 text-xs text-destructive">{{ props.errorMessage }}</p>
      </CardContent>
    </Card>
  </div>
</template>
