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
  <aside class="flex h-full flex-col items-center gap-6 rounded-2xl border border-white/[0.08] bg-card/40 px-3 py-4 backdrop-blur-3xl shadow-xl shadow-black/20">
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
        <Tooltip v-if="props.tasksCount > 0">
          <TooltipTrigger as-child>
            <Button
              class="relative flex h-12 w-12 items-center justify-center rounded-2xl border text-sm font-semibold transition"
              variant="ghost"
              :class="
                props.tasksOpen
                  ? 'border-foreground/70 bg-foreground/5 text-foreground'
                  : 'border-border/60 text-muted-foreground hover:text-foreground'
              "
              @click="toggleTasks"
            >
              <ListTodo class="h-5 w-5" />
              <span
                class="absolute -right-1 -top-1 min-w-4 rounded-full bg-primary px-1 text-[10px] font-semibold leading-4 text-primary-foreground"
              >
                {{ tasksBadge }}
              </span>
            </Button>
          </TooltipTrigger>
          <TooltipContent side="right">
            {{ props.tasksOpen ? "Hide tasks" : "Show tasks" }}
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
