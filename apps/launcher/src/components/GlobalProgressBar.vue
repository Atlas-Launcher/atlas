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
const showIndeterminateProgress = computed(() => primaryPercent.value < 10);
const hasMultipleTasks = computed(() => activeTasks.value.length > 1);
const secondaryTasks = computed(() => activeTasks.value.slice(1, 4));
const hiddenSecondaryCount = computed(() => {
  return Math.max(0, activeTasks.value.length - 1 - secondaryTasks.value.length);
});
const primaryTaskHeadline = computed(() => {
  if (!primaryTask.value) {
    return "Working";
  }
  const verb = primaryTask.value.phase.toLowerCase() === "launch" ? "Launching" : "Installing";
  const packName = props.packName?.trim() || "pack";
  return `${verb} ${packName}`;
});

</script>

<template>
  <div v-if="activeTasks.length" class="fixed bottom-4 left-0 right-0 z-30">
    <div class="mx-auto w-full max-w-4xl px-6">
      <Card class="glass">
        <CardContent class="space-y-3 py-4">
          <div class="flex flex-wrap items-center justify-between gap-3">
            <div>
              <div class="text-xs uppercase tracking-widest text-muted-foreground">Active tasks</div>
              <div class="text-sm font-semibold text-foreground">
                {{ primaryTaskHeadline }}
              </div>
            </div>
            <div v-if="hasMultipleTasks" class="text-xs text-muted-foreground">
              {{ activeTasks.length }} task{{ activeTasks.length === 1 ? "" : "s" }} running
            </div>
          </div>
          <div class="relative h-2 w-full overflow-hidden rounded-full bg-secondary">
            <div
              v-if="showIndeterminateProgress"
              class="progress-stripe absolute inset-y-0 w-1/3 rounded-full"
            />
            <div
              v-else
              class="h-full rounded-full bg-primary transition-all"
              :style="{ width: `${primaryPercent}%` }"
            />
          </div>
          <div v-if="primaryTask" class="flex flex-wrap gap-2 text-xs text-muted-foreground">
            <span class="rounded-full border border-border/60 bg-card/70 px-2 py-1">
              {{ primaryTask.message }}
            </span>
            <span
              v-for="task in secondaryTasks"
              :key="task.id"
              class="rounded-full border border-border/60 bg-card/70 px-2 py-1"
            >
              {{ task.message }}
            </span>
            <span v-if="hiddenSecondaryCount > 0">
              +{{ hiddenSecondaryCount }} more
            </span>
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
