<script setup lang="ts">
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger
} from "@/components/ui/tooltip";
import { CircleQuestionMark, House, Settings } from "lucide-vue-next";
type TabKey = "library" | "settings";

const props = defineProps<{
  activeTab: TabKey;
}>();

const emit = defineEmits<{
  (event: "select", value: TabKey): void;
}>();

function selectTab(tab: TabKey) {
  emit("select", tab);
}
</script>

<template>
  <aside class="flex h-full flex-col items-center gap-4 rounded-3xl border border-border/60 bg-card/80 px-3 py-4">
    <TooltipProvider>
      <div class="flex flex-col gap-3">
        <Tooltip>
          <TooltipTrigger as-child>
            <Button
              class="flex h-11 w-11 items-center justify-center rounded-2xl border text-sm font-semibold transition"
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
              class="flex h-11 w-11 items-center justify-center rounded-2xl border text-sm font-semibold transition"
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

      <Tooltip>
        <TooltipTrigger as-child>
          <div class="mt-auto flex h-11 w-11 items-center justify-center rounded-2xl border border-border/60 text-xs font-semibold text-muted-foreground">
            <CircleQuestionMark class="h-5 w-5" />
          </div>
        </TooltipTrigger>
        <TooltipContent side="right">Help</TooltipContent>
      </Tooltip>
    </TooltipProvider>
  </aside>
</template>
