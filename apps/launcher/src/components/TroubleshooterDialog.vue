<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import Button from "./ui/button/Button.vue";
import { Sheet, SheetContent, SheetDescription, SheetHeader, SheetTitle } from "./ui/sheet";
import type {
  FixAction,
  FixResult,
  LaunchReadinessReport,
  ReadinessItem,
  TroubleshooterFinding,
  TroubleshooterReport
} from "@/types/diagnostics";

const props = defineProps<{
  open: boolean;
  gameDir: string | null;
  packId: string | null;
  channel: "dev" | "beta" | "production" | null;
  recentStatus: string;
  recentLogs: string[];
}>();

const emit = defineEmits<{
  (event: "update:open", value: boolean): void;
  (event: "status", value: string): void;
  (event: "log", value: string): void;
  (event: "relink-requested"): void;
}>();

const loading = ref(false);
const applyingFix = ref<FixAction | null>(null);
const report = ref<TroubleshooterReport | null>(null);
const beforeReadiness = ref<LaunchReadinessReport | null>(null);
const afterReadiness = ref<LaunchReadinessReport | null>(null);
const fixHistory = ref<string[]>([]);
const errorText = ref<string | null>(null);

const hasFindings = computed(() => (report.value?.findings.length ?? 0) > 0);
const topFinding = computed<TroubleshooterFinding | null>(() => {
  const findings = report.value?.findings ?? [];
  if (findings.length === 0) {
    return null;
  }
  return [...findings].sort((a, b) => b.confidence - a.confidence)[0] ?? null;
});

const topActions = computed<FixAction[]>(() => {
  const finding = topFinding.value;
  if (!finding) {
    return [];
  }
  return [...new Set(finding.suggestedActions)];
});

const readinessDiffRows = computed(() => {
  const before = beforeReadiness.value;
  const after = afterReadiness.value;
  if (!before || !after) {
    return [] as Array<{ label: string; before: boolean; after: boolean; changed: boolean }>;
  }

  const beforeByKey = new Map<string, ReadinessItem>();
  for (const item of before.checklist) {
    beforeByKey.set(item.key, item);
  }

  return after.checklist.map((item) => {
    const previous = beforeByKey.get(item.key);
    const beforeReady = previous?.ready ?? false;
    return {
      label: item.label,
      before: beforeReady,
      after: item.ready,
      changed: beforeReady !== item.ready
    };
  });
});

const recentLogsForRun = computed(() => props.recentLogs.slice(0, 120));

function actionLabel(action: FixAction): string {
  switch (action) {
    case "relinkAccount":
      return "Relink account";
    case "setSafeMemory":
      return "Set safe memory";
    case "resyncPack":
      return "Resync pack";
    case "repairRuntime":
      return "Repair runtime";
    case "fullRepair":
      return "Run full repair";
  }
}

function closeDialog() {
  emit("update:open", false);
}

async function refreshTroubleshooter() {
  loading.value = true;
  errorText.value = null;
  try {
    const next = await invoke<TroubleshooterReport>("run_troubleshooter", {
      gameDir: props.gameDir,
      recentStatus: props.recentStatus || null,
      recentLogs: recentLogsForRun.value
    });
    report.value = next;
    if (!beforeReadiness.value) {
      beforeReadiness.value = next.readiness;
    }
    if (afterReadiness.value) {
      afterReadiness.value = next.readiness;
    }
  } catch (err) {
    const message = `Troubleshooter failed: ${String(err)}`;
    errorText.value = message;
    emit("status", message);
    emit("log", message);
  } finally {
    loading.value = false;
  }
}

async function applyFix(action: FixAction) {
  if (applyingFix.value) {
    return;
  }
  applyingFix.value = action;
  errorText.value = null;
  try {
    const result = await invoke<FixResult>("apply_fix", {
      action,
      gameDir: props.gameDir,
      packId: props.packId,
      channel: props.channel
    });
    const statusLine = result.message;
    emit("status", statusLine);
    emit("log", `[Troubleshooter] ${statusLine}`);
    fixHistory.value = [statusLine, ...fixHistory.value].slice(0, 20);

    if (action === "relinkAccount") {
      emit("relink-requested");
    }

    const readiness = await invoke<LaunchReadinessReport>("get_launch_readiness", {
      gameDir: props.gameDir
    });
    afterReadiness.value = readiness;

    await refreshTroubleshooter();
  } catch (err) {
    const message = `Fix failed (${actionLabel(action)}): ${String(err)}`;
    errorText.value = message;
    emit("status", message);
    emit("log", message);
  } finally {
    applyingFix.value = null;
  }
}

watch(
  () => props.open,
  async (value) => {
    if (!value) {
      return;
    }
    report.value = null;
    beforeReadiness.value = null;
    afterReadiness.value = null;
    fixHistory.value = [];
    await refreshTroubleshooter();
  }
);
</script>

<template>
  <Sheet :open="props.open" @update:open="emit('update:open', $event)">
    <SheetContent
      side="right"
      class="w-[96vw] max-w-2xl sm:max-w-2xl overflow-y-auto border-l border-border/60"
    >
      <SheetHeader>
        <SheetTitle>Troubleshooter</SheetTitle>
        <SheetDescription>
          Diagnose launch issues and apply guided one-click fixes.
        </SheetDescription>
      </SheetHeader>

      <div class="mt-6 space-y-5">
        <div v-if="loading" class="text-sm text-muted-foreground">Analyzing current state...</div>

        <div v-if="errorText" class="rounded-xl border border-destructive/40 bg-destructive/10 p-3 text-sm text-destructive">
          {{ errorText }}
        </div>

        <section v-if="!loading && topFinding" class="space-y-3 rounded-2xl border border-border/60 bg-card/70 p-4">
          <div class="flex items-start justify-between gap-3">
            <div>
              <p class="text-xs uppercase tracking-widest text-muted-foreground">Top finding</p>
              <h3 class="mt-1 text-base font-semibold text-foreground">{{ topFinding.title }}</h3>
              <p class="mt-2 text-sm text-muted-foreground">{{ topFinding.detail }}</p>
            </div>
            <span class="rounded-full border border-border/70 px-2 py-1 text-xs text-muted-foreground">
              {{ topFinding.confidence }}% confidence
            </span>
          </div>

          <div class="flex flex-wrap gap-2">
            <Button
              v-for="action in topActions"
              :key="action"
              size="sm"
              :disabled="!!applyingFix"
              @click="applyFix(action)"
            >
              <span v-if="applyingFix === action">Applying...</span>
              <span v-else>{{ actionLabel(action) }}</span>
            </Button>
          </div>
        </section>

        <section v-if="!loading && !hasFindings" class="rounded-2xl border border-emerald-600/30 bg-emerald-600/10 p-4 text-sm text-emerald-700 dark:text-emerald-300">
          No active findings. Launch readiness looks good.
        </section>

        <section v-if="report && report.findings.length > 1" class="space-y-2">
          <h4 class="text-xs uppercase tracking-widest text-muted-foreground">Other findings</h4>
          <ul class="space-y-2">
            <li
              v-for="finding in report.findings.filter((entry) => entry.code !== topFinding?.code)"
              :key="finding.code"
              class="rounded-xl border border-border/50 bg-card/60 p-3"
            >
              <div class="flex items-center justify-between gap-2">
                <p class="text-sm font-medium text-foreground">{{ finding.title }}</p>
                <span class="text-xs text-muted-foreground">{{ finding.confidence }}%</span>
              </div>
              <p class="mt-1 text-xs text-muted-foreground">{{ finding.detail }}</p>
            </li>
          </ul>
        </section>

        <section v-if="beforeReadiness" class="space-y-2 rounded-2xl border border-border/50 bg-card/60 p-4">
          <h4 class="text-xs uppercase tracking-widest text-muted-foreground">Readiness snapshot</h4>
          <ul class="space-y-1.5 text-sm">
            <li v-for="item in beforeReadiness.checklist" :key="item.key" class="flex items-center justify-between gap-3">
              <span class="text-foreground">{{ item.label }}</span>
              <span :class="item.ready ? 'text-emerald-600 dark:text-emerald-300' : 'text-amber-600 dark:text-amber-300'">
                {{ item.ready ? "Ready" : "Needs attention" }}
              </span>
            </li>
          </ul>
        </section>

        <section v-if="readinessDiffRows.length > 0" class="space-y-2 rounded-2xl border border-border/50 bg-card/60 p-4">
          <h4 class="text-xs uppercase tracking-widest text-muted-foreground">Before / after</h4>
          <ul class="space-y-1.5 text-sm">
            <li v-for="row in readinessDiffRows" :key="row.label" class="grid grid-cols-[1fr_auto_auto] items-center gap-3">
              <span class="text-foreground">{{ row.label }}</span>
              <span :class="row.before ? 'text-emerald-600 dark:text-emerald-300' : 'text-amber-600 dark:text-amber-300'">
                {{ row.before ? "Ready" : "Not ready" }}
              </span>
              <span :class="row.after ? 'text-emerald-600 dark:text-emerald-300' : 'text-amber-600 dark:text-amber-300'">
                {{ row.after ? "Ready" : "Not ready" }}
              </span>
            </li>
          </ul>
          <p class="text-xs text-muted-foreground">Rows change after a fix when readiness improves or regresses.</p>
        </section>

        <section v-if="fixHistory.length > 0" class="space-y-2">
          <h4 class="text-xs uppercase tracking-widest text-muted-foreground">Applied fixes</h4>
          <ul class="space-y-1.5 text-xs text-muted-foreground">
            <li v-for="(entry, index) in fixHistory" :key="`${entry}:${index}`">{{ entry }}</li>
          </ul>
        </section>

        <div class="flex justify-end pt-2">
          <Button variant="secondary" @click="closeDialog">Close</Button>
        </div>
      </div>
    </SheetContent>
  </Sheet>
</template>
