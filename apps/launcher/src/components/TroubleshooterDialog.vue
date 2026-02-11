<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import Button from "./ui/button/Button.vue";
import { Sheet, SheetContent, SheetDescription, SheetHeader, SheetTitle } from "./ui/sheet";
import type {
  FixAction,
  FixResult,
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
const fixHistory = ref<string[]>([]);
const errorText = ref<string | null>(null);

const AUTH_FINDING_CODES = new Set([
  "atlas_not_signed_in",
  "microsoft_not_signed_in",
  "account_link_mismatch"
]);

const findings = computed(() =>
  (report.value?.findings ?? []).filter((finding) => !AUTH_FINDING_CODES.has(finding.code))
);
const hasFindings = computed(() => findings.value.length > 0);
const topFinding = computed<TroubleshooterFinding | null>(() => {
  if (findings.value.length === 0) {
    return null;
  }
  return [...findings.value].sort((a, b) => b.confidence - a.confidence)[0] ?? null;
});

const topActions = computed<FixAction[]>(() => {
  const finding = topFinding.value;
  if (!finding) {
    return [];
  }
  return [...new Set(finding.suggestedActions)];
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
    emit("log", `[TroubleshooterDialog] run_troubleshooter returned: ${JSON.stringify(next)}`);
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
    emit("log", "[TroubleshooterDialog] props.open changed -> true");
    report.value = null;
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
          Diagnose installed-instance launch/runtime problems and apply one-click fixes.
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
          No launch/runtime findings detected for this installed instance.
        </section>

        <section v-if="findings.length > 1" class="space-y-2">
          <h4 class="text-xs uppercase tracking-widest text-muted-foreground">Other findings</h4>
          <ul class="space-y-2">
            <li
              v-for="finding in findings.filter((entry) => entry.code !== topFinding?.code)"
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
