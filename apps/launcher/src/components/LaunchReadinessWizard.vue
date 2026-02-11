<script setup lang="ts">
import { computed } from "vue";
import { AlertCircle, CheckCircle2, Wrench } from "lucide-vue-next";
import Button from "./ui/button/Button.vue";
import Card from "./ui/card/Card.vue";
import CardContent from "./ui/card/CardContent.vue";
import CardDescription from "./ui/card/CardDescription.vue";
import CardFooter from "./ui/card/CardFooter.vue";
import CardHeader from "./ui/card/CardHeader.vue";
import CardTitle from "./ui/card/CardTitle.vue";
import type { LaunchReadinessReport } from "@/types/diagnostics";

const props = defineProps<{
  open: boolean;
  readiness: LaunchReadinessReport | null;
  working: boolean;
  nextActionLabels: Partial<Record<string, string>>;
}>();

const emit = defineEmits<{
  (event: "close"): void;
  (event: "complete"): void;
  (event: "action", key: string): void;
}>();

const checklist = computed(() => props.readiness?.checklist ?? []);
const allReady = computed(() => checklist.value.length > 0 && checklist.value.every((item) => item.ready));
</script>

<template>
  <div v-if="props.open" class="fixed inset-0 z-[70] bg-black/45 backdrop-blur-[2px] p-4 md:p-6">
    <div class="mx-auto flex h-full max-w-3xl items-center justify-center">
      <Card class="glass w-full max-h-full overflow-hidden">
        <CardHeader>
          <CardTitle>Launch Readiness Wizard</CardTitle>
          <CardDescription>
            Complete setup checks before launching Minecraft.
          </CardDescription>
        </CardHeader>
        <CardContent class="space-y-4 overflow-y-auto">
          <div v-if="!props.readiness" class="rounded-xl border border-border/60 bg-background/40 p-3 text-sm text-muted-foreground">
            Loading readiness status...
          </div>
          <div
            v-for="item in checklist"
            :key="item.key"
            class="rounded-xl border bg-background/30 p-4"
            :class="item.ready ? 'border-emerald-500/40' : 'border-amber-500/40'"
          >
            <div class="flex items-start justify-between gap-3">
              <div class="min-w-0">
                <div class="flex items-center gap-2 text-sm font-semibold">
                  <CheckCircle2 v-if="item.ready" class="h-4 w-4 text-emerald-500" />
                  <AlertCircle v-else class="h-4 w-4 text-amber-500" />
                  <span>{{ item.label }}</span>
                </div>
                <p v-if="item.detail" class="mt-1 text-xs text-muted-foreground">
                  {{ item.detail }}
                </p>
              </div>
              <Button
                v-if="!item.ready && props.nextActionLabels[item.key]"
                variant="secondary"
                size="sm"
                :disabled="props.working"
                @click="emit('action', item.key)"
              >
                <Wrench class="mr-1 h-3.5 w-3.5" />
                {{ props.nextActionLabels[item.key] }}
              </Button>
            </div>
          </div>
        </CardContent>
        <CardFooter class="flex items-center justify-between gap-3">
          <Button variant="ghost" :disabled="props.working" @click="emit('close')">
            Not now
          </Button>
          <Button
            :disabled="props.working || !allReady"
            @click="emit('complete')"
          >
            Continue
          </Button>
        </CardFooter>
      </Card>
    </div>
  </div>
</template>
