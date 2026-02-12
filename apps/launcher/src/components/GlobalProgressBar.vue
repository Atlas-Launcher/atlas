<script setup lang="ts">
import { computed } from "vue";
import Card from "./ui/card/Card.vue";
import CardContent from "./ui/card/CardContent.vue";
import type { ActiveTask } from "@/lib/useStatus";

const props = defineProps<{
  tasks: ActiveTask[];
  packName?: string | null;
}>();

const activeTasks = computed(() => {
  return [...props.tasks].sort((a, b) => a.startedAt - b.startedAt);
});

const primaryTask = computed(() => activeTasks.value[0] ?? null);
const primaryPercent = computed(() => {
  return Math.min(100, Math.max(0, primaryTask.value?.percent ?? 0));
});
const showIndeterminateProgress = computed(
  () => (primaryTask.value?.indeterminate ?? false) || primaryPercent.value < 5
);
const hasMultipleTasks = computed(() => activeTasks.value.length > 1);
const secondaryTasks = computed(() => activeTasks.value.slice(1, 4));
const hiddenSecondaryCount = computed(() => {
  return Math.max(0, activeTasks.value.length - 1 - secondaryTasks.value.length);
});
const primaryTaskStage = computed(() => {
  if (!primaryTask.value) {
    return "No active tasks";
  }
  return primaryTask.value.stageLabel || "Preparing files";
});
const primaryTaskHeadline = computed(() => {
  if (!primaryTask.value) {
    return "No active tasks";
  }
  const packName = props.packName?.trim();
  if (!packName) {
    return primaryTaskStage.value;
  }
  return `${primaryTaskStage.value} ${packName}`;
});
const primaryTaskStatusText = computed(() => {
  if (!primaryTask.value) {
    return "";
  }
  if (primaryTask.value.indeterminate && primaryTask.value.statusText.trim().length === 0) {
    return "Working through setup. Time remaining will appear once progress is measurable.";
  }
  return primaryTask.value.statusText;
});
const primaryTaskEtaText = computed(() => {
  const eta = primaryTask.value?.etaSeconds ?? null;
  if (!eta || eta <= 0) {
    if (primaryTask.value?.indeterminate) {
      return "Estimating time remaining...";
    }
    return null;
  }
  const minutes = Math.floor(eta / 60);
  const seconds = eta % 60;
  if (minutes <= 0) {
    return `ETA ${seconds}s`;
  }
  if (seconds === 0) {
    return `ETA ${minutes}m`;
  }
  return `ETA ${minutes}m ${seconds}s`;
});

</script>

<template>
  <div class="fixed bottom-4 left-0 right-0 z-30 transition-all duration-500 ease-in-out translate-y-0 pointer-events-none">
    <div class="mx-auto w-full max-w-4xl px-6">
      <Card class="glass rounded-2xl border-none bg-transparent shadow-none pointer-events-auto">
        <CardContent class="space-y-3 py-4">
          <div class="flex flex-wrap items-center justify-between gap-3">
            <div>
              <div class="text-xs uppercase tracking-widest text-muted-foreground/60 mb-2">Task Center</div>
              <div class="text-sm font-semibold text-foreground">
                {{ primaryTaskHeadline }}
              </div>
              <div v-if="primaryTaskStatusText" class="mt-1 text-xs text-muted-foreground">
                {{ primaryTaskStatusText }}
              </div>
            </div>
            <div v-if="hasMultipleTasks" class="text-xs text-muted-foreground">
              {{ activeTasks.length }} tasks running
            </div>
            <div v-else-if="primaryTaskEtaText" class="text-xs text-muted-foreground">
              {{ primaryTaskEtaText }}
            </div>
            <div v-else-if="!activeTasks.length" class="text-xs text-muted-foreground/40 italic">
              No active tasks
            </div>
          </div>
          
          <div class="relative h-1.5 w-full overflow-hidden rounded-full bg-secondary/30">
            <template v-if="activeTasks.length">
              <div
                v-if="showIndeterminateProgress"
                class="progress-stripe absolute inset-y-0 w-1/3 rounded-full"
              />
              <div
                v-else
                class="h-full rounded-full bg-primary transition-all duration-300"
                :style="{ width: `${primaryPercent}%` }"
              />
            </template>
            <div v-else class="h-full w-0 bg-primary/20 transition-all duration-300" />
          </div>

          <div v-if="activeTasks.length" class="flex flex-wrap gap-2 text-xs text-muted-foreground">
            <span class="glass rounded-full px-3 py-1 text-[10px] font-bold uppercase tracking-wider">
              {{ primaryTask.stageLabel }}
            </span>
            <span
              v-for="task in secondaryTasks"
              :key="task.id"
              class="glass rounded-full px-3 py-1 text-[10px] font-bold uppercase tracking-wider"
            >
              {{ task.stageLabel || task.message }}
            </span>
            <span v-if="hiddenSecondaryCount > 0" class="self-center px-1">
              +{{ hiddenSecondaryCount }} more
            </span>
          </div>
          <div v-else class="text-xs text-muted-foreground/40">
          </div>
        </CardContent>
      </Card>
    </div>
  </div>
</template>

<style scoped>
@keyframes progress-stripe-sweep {
  from {
    transform: translateX(-140%);
  }

  to {
    transform: translateX(360%);
  }
}

.progress-stripe {
  animation: progress-stripe-sweep 1.2s linear infinite;
  background: linear-gradient(
    90deg,
    transparent 0%,
    hsl(var(--primary) / 0.2) 35%,
    hsl(var(--primary) / 0.85) 50%,
    hsl(var(--primary) / 0.2) 65%,
    transparent 100%
  );
}
</style>
