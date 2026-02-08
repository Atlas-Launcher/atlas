<script setup lang="ts">
import { computed } from "vue";
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger
} from "@/components/ui/tooltip";
import { CircleQuestionMark, House, ListTodo, Settings } from "lucide-vue-next";
type TabKey = "library" | "settings";

const props = defineProps<{
  activeTab: TabKey;
  tasksCount: number;
  tasksOpen: boolean;
}>();

const emit = defineEmits<{
  (event: "select", value: TabKey): void;
  (event: "toggle-tasks"): void;
}>();

function selectTab(tab: TabKey) {
  emit("select", tab);
}

function toggleTasks() {
  emit("toggle-tasks");
}

const tasksBadge = computed(() => (props.tasksCount > 9 ? "9+" : String(props.tasksCount)));
</script>

<template>
  <aside class="glass relative z-40 flex h-full flex-col items-center gap-6 rounded-2xl px-3 py-4">
    <TooltipProvider>
      <div class="flex flex-col gap-4">
        <Tooltip>
          <TooltipTrigger as-child>
            <Button
              class="flex h-12 w-12 items-center justify-center rounded-2xl border text-sm font-semibold transition"
              variant="ghost"
              :class="
                props.activeTab === 'library'
                  ? 'border-foreground/70 bg-foreground/5 text-foreground'
                  : 'border-border/60 text-muted-foreground hover:text-foreground'
              "
              @click="selectTab('library')"
            >
              <House class="h-5 w-5" />
            </Button>
          </TooltipTrigger>
          <TooltipContent side="right">Home</TooltipContent>
        </Tooltip>

        <Tooltip>
          <TooltipTrigger as-child>
            <Button
              class="flex h-12 w-12 items-center justify-center rounded-2xl border text-sm font-semibold transition"
              variant="ghost"
              :class="
                props.activeTab === 'settings'
                  ? 'border-foreground/70 bg-foreground/5 text-foreground'
                  : 'border-border/60 text-muted-foreground hover:text-foreground'
              "
              @click="selectTab('settings')"
            >
              <Settings class="h-5 w-5" />
            </Button>
          </TooltipTrigger>
          <TooltipContent side="right">Settings</TooltipContent>
        </Tooltip>
      </div>

      <div class="mt-auto flex flex-col gap-4">
        <Tooltip>
          <TooltipTrigger as-child>
            <Button
              class="relative flex h-12 w-12 items-center justify-center rounded-2xl border text-sm font-semibold transition overflow-visible"
              variant="ghost"
              :class="
                props.tasksOpen
                  ? 'border-foreground/70 bg-foreground/5 text-foreground'
                  : 'border-border/60 text-muted-foreground hover:text-foreground'
              "
              @click="toggleTasks"
            >
              <ListTodo class="h-5 w-5" />
              
              <!-- Edge Flow Loading Indicator (Mathematically Perfect 1px Inset) -->
              <svg 
                v-if="props.tasksCount > 0"
                key="task-loading-svg"
                class="absolute inset-0 !size-12 pointer-events-none overflow-visible" 
                fill="none"
                viewBox="0 0 48 48"
              >
                <path 
                  d="M 24,1 L 32,1 A 15,15 0 0 1 47,16 L 47,32 A 15,15 0 0 1 32,47 L 16,47 A 15,15 0 0 1 1,32 L 1,16 A 15,15 0 0 1 16,1 Z" 
                  class="stroke-primary flow-animation"
                  stroke-width="2.5"
                  stroke-dasharray="0.3 0.7"
                  stroke-linecap="round"
                  pathLength="1"
                />
              </svg>

              <span
                v-if="props.tasksCount > 0"
                key="task-loading-badge"
                class="absolute -right-1 -top-1 min-w-4 rounded-full bg-primary px-1 text-[10px] font-semibold leading-4 text-primary-foreground shadow-sm"
              >
                {{ tasksBadge }}
              </span>
            </Button>
          </TooltipTrigger>
          <TooltipContent side="right">
            {{ props.tasksOpen ? "Hide Task Center" : "Open Task Center" }}
          </TooltipContent>
        </Tooltip>

        <Tooltip>
          <TooltipTrigger as-child>
            <div class="flex h-12 w-12 items-center justify-center rounded-2xl border border-border/60 text-xs font-semibold text-muted-foreground">
              <CircleQuestionMark class="h-5 w-5" />
            </div>
          </TooltipTrigger>
          <TooltipContent side="right">Help</TooltipContent>
        </Tooltip>
      </div>
    </TooltipProvider>
  </aside>
</template>

<style scoped>
@keyframes edgeFlow {
  from {
    stroke-dashoffset: 1;
  }
  to {
    stroke-dashoffset: 0;
  }
}

.flow-animation {
  animation: edgeFlow 2s linear infinite;
}
</style>
