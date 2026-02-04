<script setup lang="ts">
import { computed } from "vue";
import Card from "./ui/card/Card.vue";
import CardContent from "./ui/card/CardContent.vue";
import Progress from "./ui/progress/Progress.vue";
import type { ActiveTask } from "@/lib/useStatus";

const props = defineProps<{
  tasks: ActiveTask[];
}>();

const activeTasks = computed(() => {
  return [...props.tasks].sort((a, b) => a.startedAt - b.startedAt);
});

const primaryTask = computed(() => activeTasks.value[0] ?? null);

const visibleTasks = computed(() => activeTasks.value.slice(0, 3));
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
                {{ primaryTask?.message ?? "Working" }}
              </div>
            </div>
            <div class="text-xs text-muted-foreground">
              {{ activeTasks.length }} task{{ activeTasks.length === 1 ? "" : "s" }} running
            </div>
          </div>
          <Progress :model-value="primaryTask?.percent ?? 0" />
          <div class="flex flex-wrap gap-2 text-xs text-muted-foreground">
            <span
              v-for="task in visibleTasks"
              :key="task.id"
              class="rounded-full border border-border/60 bg-card/70 px-2 py-1"
            >
              {{ task.phase }} · {{ task.message }}
            </span>
            <span v-if="activeTasks.length > visibleTasks.length">
              +{{ activeTasks.length - visibleTasks.length }} more
            </span>
          </div>
        </CardContent>
      </Card>
    </div>
  </div>
</template>
