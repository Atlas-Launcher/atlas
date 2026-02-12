<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  AlertCircle,
  CheckCircle2,
  LoaderCircle,
  Link2,
  ShieldCheck,
  Wrench,
  X
} from "lucide-vue-next";
import Button from "./ui/button/Button.vue";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger
} from "./ui/dropdown-menu";
import Card from "./ui/card/Card.vue";
import CardContent from "./ui/card/CardContent.vue";
import CardDescription from "./ui/card/CardDescription.vue";
import CardHeader from "./ui/card/CardHeader.vue";
import CardTitle from "./ui/card/CardTitle.vue";
import type {
  FixAction,
  LaunchReadinessReport,
  SupportBundleResult,
  TroubleshooterFinding,
  TroubleshooterReport
} from "@/types/diagnostics";
import type { LauncherLinkSession } from "@/types/auth";

const props = defineProps<{
  open: boolean;
  mode: "readiness" | "recovery";
  readiness: LaunchReadinessReport | null;
  atlasSignedIn: boolean;
  microsoftSignedIn: boolean;
  isSigningIn: boolean;
  linkSession: LauncherLinkSession | null;
  hubUrl: string;
  working: boolean;
  nextActionLabels: Partial<Record<string, string>>;
  gameDir: string | null;
  packId: string | null;
  channel: "dev" | "beta" | "production" | null;
  recentStatus: string;
  recentLogs: string[];
}>();

const emit = defineEmits<{
  (event: "close"): void;
  (event: "complete"): void;
  (event: "sign-out", scope: "microsoft" | "all"): void;
  (event: "action", key: string): void;
  (event: "status", value: string): void;
  (event: "log", value: string): void;
  (event: "relink-requested"): void;
  (event: "retry-launch"): void;
}>();

const mode = ref<"readiness" | "recovery">("readiness");
const copyStatus = ref<string | null>(null);
const showLinkCode = ref(false);
const closeButtonRef = ref<HTMLButtonElement | null>(null);

const loading = ref(false);
const applyingFix = ref<FixAction | null>(null);
const report = ref<TroubleshooterReport | null>(null);
const fixHistory = ref<string[]>([]);
const errorText = ref<string | null>(null);
const supportBundle = ref<SupportBundleResult | null>(null);
const supportBundleLoading = ref(false);
const supportBundleError = ref<string | null>(null);

const LOGIN_KEYS = new Set(["atlasLogin", "microsoftLogin", "accountLink"]);
const AUTH_FINDING_CODES = new Set([
  "atlas_not_signed_in",
  "microsoft_not_signed_in",
  "account_link_mismatch"
]);

const checklist = computed(() => props.readiness?.checklist ?? []);
const blockingItems = computed(() => checklist.value.filter((item) => !item.ready));
const activeBlocking = computed(() => blockingItems.value[0] ?? null);
const allReady = computed(() => props.readiness?.readyToLaunch ?? false);
const signedIn = computed(() => props.atlasSignedIn || props.microsoftSignedIn);

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

const linkUrl = computed(() => {
  if (!props.linkSession) {
    return null;
  }
  const base = props.hubUrl.replace(/\/$/, "");
  return `${base}/link/launcher?code=${encodeURIComponent(props.linkSession.linkCode)}`;
});

const accountLinkStatus = computed(() => {
  const item = activeBlocking.value;
  if (!item || item.key !== "accountLink") {
    return null;
  }

  if (props.isSigningIn) {
    return "Your browser opened for sign-in. Complete it there to continue.";
  }

  if (props.linkSession) {
    return "Waiting for Atlas to confirm sign-in. This usually takes a few seconds.";
  }

  return "Select continue to finish account linking in your browser.";
});

function actionLabel(action: FixAction): string {
  switch (action) {
    case "relinkAccount":
      return "Relink account";
    case "setSafeMemory":
      return "Use safe memory settings";
    case "resyncPack":
      return "Sync pack again";
    case "repairRuntime":
      return "Repair runtime";
    case "fullRepair":
      return "Run full repair";
  }
}

function resetRecoveryState() {
  loading.value = false;
  applyingFix.value = null;
  report.value = null;
  fixHistory.value = [];
  errorText.value = null;
  supportBundle.value = null;
  supportBundleError.value = null;
}

async function refreshTroubleshooter() {
  loading.value = true;
  errorText.value = null;
  try {
    const next = await invoke<TroubleshooterReport>("run_troubleshooter", {
      gameDir: props.gameDir,
      recentStatus: props.recentStatus || null,
      recentLogs: props.recentLogs.slice(0, 120)
    });
    report.value = next;
    emit("log", `[LaunchAssist] run_troubleshooter returned: ${JSON.stringify(next)}`);
  } catch (err) {
    const message = `Troubleshooter failed: ${String(err)}`;
    errorText.value = message;
    emit("status", message);
    emit("log", message);
  } finally {
    loading.value = false;
  }
}

async function applyFixAction(action: FixAction, retryAfter: boolean = false) {
  if (applyingFix.value) {
    return;
  }
  applyingFix.value = action;
  errorText.value = null;
  try {
    const result = await invoke<{ message: string }>("apply_fix", {
      action,
      gameDir: props.gameDir,
      packId: props.packId,
      channel: props.channel
    });

    const statusLine = result.message;
    emit("status", statusLine);
    emit("log", `[LaunchAssist] ${statusLine}`);
    fixHistory.value = [statusLine, ...fixHistory.value].slice(0, 20);

    if (action === "relinkAccount") {
      emit("relink-requested");
    }

    await refreshTroubleshooter();

    if (retryAfter) {
      emit("retry-launch");
    }
  } catch (err) {
    const message = `Fix failed (${actionLabel(action)}): ${String(err)}`;
    errorText.value = message;
    emit("status", message);
    emit("log", message);
  } finally {
    applyingFix.value = null;
  }
}

async function fixAndRetry() {
  const primary = topActions.value[0];
  if (!primary) {
    emit("retry-launch");
    return;
  }
  await applyFixAction(primary, true);
}

async function generateSupportBundle() {
  supportBundleLoading.value = true;
  supportBundleError.value = null;
  try {
    const bundle = await invoke<SupportBundleResult>("create_support_bundle", {
      gameDir: props.gameDir,
      recentStatus: props.recentStatus || null,
      recentLogs: props.recentLogs.slice(0, 120)
    });
    supportBundle.value = bundle;
    emit("status", "Support bundle generated.");
    emit("log", `[LaunchAssist] support bundle created at ${bundle.bundleDir}`);
  } catch (err) {
    supportBundleError.value = `Support bundle failed: ${String(err)}`;
  } finally {
    supportBundleLoading.value = false;
  }
}

async function copyLinkCode() {
  if (!props.linkSession) {
    return;
  }
  try {
    await navigator.clipboard.writeText(props.linkSession.linkCode);
    copyStatus.value = "Copied.";
  } catch {
    copyStatus.value = "Copy failed.";
  }
}

async function openLinkPage() {
  if (!linkUrl.value) {
    return;
  }
  await openUrl(linkUrl.value);
}

function revealLinkCode() {
  showLinkCode.value = true;
}

function openRecovery() {
  mode.value = "recovery";
  void refreshTroubleshooter();
}

function runPrimaryReadinessAction() {
  const current = activeBlocking.value;
  if (!current) {
    emit("complete");
    return;
  }
  const mapped = props.nextActionLabels[current.key];
  if (mapped) {
    emit("action", current.key);
    return;
  }
  openRecovery();
}

function closeWizard() {
  emit("close");
}

watch(
  () => props.open,
  (open) => {
    if (!open) {
      return;
    }
    void nextTick(() => {
      closeButtonRef.value?.focus();
    });
    mode.value = props.mode;
    copyStatus.value = null;
    showLinkCode.value = false;
    if (mode.value === "recovery") {
      resetRecoveryState();
      void refreshTroubleshooter();
    }
  }
);

watch(
  () => props.mode,
  (next) => {
    if (!props.open) {
      return;
    }
    if (mode.value !== next) {
      mode.value = next;
      if (next === "recovery") {
        resetRecoveryState();
        void refreshTroubleshooter();
      }
    }
  }
);
</script>

<template>
  <div
    v-if="props.open"
    class="fixed inset-0 z-[70] bg-black/55 backdrop-blur-[6px] p-4 md:p-6"
    tabindex="-1"
    @keydown.esc.prevent="closeWizard"
  >
    <div class="mx-auto flex h-full max-w-4xl items-center justify-center">
      <Card class="glass relative w-full max-h-full overflow-hidden bg-background/95">
        <button
          ref="closeButtonRef"
          class="absolute right-6 top-5 inline-flex h-8 w-8 items-center justify-center rounded-md border border-border/70 text-muted-foreground hover:bg-background/60 hover:text-foreground"
          type="button"
          :disabled="props.working"
          aria-label="Close"
          @click="closeWizard"
        >
          <X class="h-4 w-4" />
        </button>

        <CardHeader class="space-y-3 pr-14">
          <CardTitle>Launch Assist</CardTitle>
          <CardDescription>Check readiness, fix issues, and retry from one guided flow.</CardDescription>
          <div class="flex gap-2 pt-1">
            <Button
              size="sm"
              variant="outline"
              :class="mode === 'readiness' ? 'border-primary text-primary' : ''"
              @click="mode = 'readiness'"
            >
              Readiness
            </Button>
            <Button
              size="sm"
              variant="outline"
              :class="mode === 'recovery' ? 'border-primary text-primary' : ''"
              @click="openRecovery"
            >
              Recovery
            </Button>
          </div>
        </CardHeader>

        <CardContent class="space-y-4 overflow-y-auto max-h-[70vh]">
          <template v-if="mode === 'readiness'">
            <div v-if="!props.readiness" class="rounded-xl border border-border/60 bg-background/40 p-3 text-sm text-muted-foreground">
              Checking readiness...
            </div>

            <div v-if="allReady" class="rounded-xl border border-emerald-500/40 bg-emerald-500/10 p-4">
              <div class="flex items-center gap-2 text-sm font-semibold text-emerald-700 dark:text-emerald-300">
                <ShieldCheck class="h-4 w-4" />
                Ready to launch
              </div>
              <p class="mt-1 text-xs text-muted-foreground">All launch blockers are resolved.</p>
            </div>

            <div
              v-for="item in checklist"
              :key="item.key"
              class="rounded-xl border bg-background/30 p-4"
              :class="item.ready ? 'border-emerald-500/40' : item.key === activeBlocking?.key ? 'border-amber-500/60' : 'border-border/50'"
            >
              <div class="flex items-start justify-between gap-3">
                <div class="min-w-0">
                  <div class="flex items-center gap-2 text-sm font-semibold">
                    <CheckCircle2 v-if="item.ready" class="h-4 w-4 text-emerald-500" />
                    <AlertCircle v-else class="h-4 w-4 text-amber-500" />
                    <span>{{ item.label }}</span>
                  </div>
                  <p v-if="item.detail" class="mt-1 text-xs text-muted-foreground">{{ item.detail }}</p>
                </div>
                <span v-if="!item.ready" class="rounded-full border border-amber-500/30 bg-amber-500/10 px-2 py-0.5 text-[10px] uppercase tracking-wide text-amber-700 dark:text-amber-300">
                  blocked
                </span>
              </div>

              <div
                v-if="item.key === 'accountLink' && !item.ready && item.key === activeBlocking?.key && props.linkSession && showLinkCode"
                class="mt-3 space-y-2 rounded-lg border border-border/60 bg-card/60 p-3"
              >
                <div class="text-[11px] uppercase tracking-widest text-muted-foreground">Link code</div>
                <div class="text-lg font-semibold tracking-[0.15em] text-foreground">{{ props.linkSession.linkCode }}</div>
                <div class="text-xs text-muted-foreground">Expires at {{ props.linkSession.expiresAt }}</div>
                <div class="flex flex-wrap gap-2">
                  <Button size="sm" variant="outline" :disabled="props.working" @click="copyLinkCode">
                    <Link2 class="mr-1 h-3.5 w-3.5" />
                    Copy code
                  </Button>
                  <Button size="sm" variant="outline" :disabled="props.working" @click="openLinkPage">
                    Open link page
                  </Button>
                </div>
                <p v-if="copyStatus" class="text-xs text-muted-foreground">{{ copyStatus }}</p>
              </div>

              <div
                v-if="item.key === 'accountLink' && !item.ready && item.key === activeBlocking?.key && props.linkSession && !showLinkCode"
                class="mt-4"
              >
                <Button
                  variant="link"
                  size="sm"
                  class="p-0 text-xs text-muted-foreground underline-offset-4 hover:underline"
                  @click="revealLinkCode"
                >
                  Browser did not open? Show code.
                </Button>
              </div>
            </div>

            <p
              v-if="activeBlocking?.key === 'accountLink' && accountLinkStatus"
              class="text-xs text-muted-foreground"
            >
              {{ accountLinkStatus }}
            </p>

            <div class="flex flex-wrap gap-2 pt-2">
              <Button
                v-if="activeBlocking"
                :disabled="props.working"
                @click="runPrimaryReadinessAction"
              >
                <Wrench class="mr-1 h-3.5 w-3.5" />
                {{ props.nextActionLabels[activeBlocking.key] ?? "Open guided recovery" }}
              </Button>
              <Button v-else variant="secondary" :disabled="props.working" @click="emit('complete')">
                Continue
              </Button>
            </div>
          </template>

          <template v-else>
            <div v-if="loading" class="text-sm text-muted-foreground">Analyzing current state...</div>

            <div
              v-if="errorText"
              class="rounded-xl border border-destructive/40 bg-destructive/10 p-3 text-sm text-destructive"
              role="alert"
            >
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
                  @click="applyFixAction(action)"
                >
                  <span v-if="applyingFix === action">Applying...</span>
                  <span v-else>{{ actionLabel(action) }}</span>
                </Button>
                <Button
                  size="sm"
                  variant="secondary"
                  :disabled="!!applyingFix"
                  @click="fixAndRetry"
                >
                  <LoaderCircle v-if="!!applyingFix" class="mr-1 h-3.5 w-3.5 animate-spin" />
                  Fix and retry launch
                </Button>
              </div>
            </section>

            <section v-if="!loading && !hasFindings" class="rounded-2xl border border-emerald-600/30 bg-emerald-600/10 p-4 text-sm text-emerald-700 dark:text-emerald-300">
              No launch or runtime issues detected.
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

            <section class="space-y-2 rounded-2xl border border-border/60 bg-card/60 p-4">
              <div class="flex items-center justify-between gap-2">
                <h4 class="text-xs uppercase tracking-widest text-muted-foreground">Support bundle</h4>
                <Button size="sm" variant="outline" :disabled="supportBundleLoading" @click="generateSupportBundle">
                  {{ supportBundleLoading ? "Generating..." : "Generate bundle" }}
                </Button>
              </div>
              <p v-if="supportBundleError" class="text-xs text-destructive" role="alert">{{ supportBundleError }}</p>
              <div v-if="supportBundle" class="space-y-1 text-xs text-muted-foreground">
                <p>Bundle: {{ supportBundle.bundleDir }}</p>
                <p>Summary: {{ supportBundle.summaryPath }}</p>
              </div>
            </section>

            <section v-if="fixHistory.length > 0" class="space-y-2">
              <h4 class="text-xs uppercase tracking-widest text-muted-foreground">Applied fixes</h4>
              <ul class="space-y-1.5 text-xs text-muted-foreground">
                <li v-for="(entry, index) in fixHistory" :key="`${entry}:${index}`">{{ entry }}</li>
              </ul>
            </section>
          </template>
        </CardContent>

        <div class="border-t border-border/50 px-6 py-3 flex items-center justify-between gap-3">
          <div class="flex items-center gap-2">
            <Button variant="outline" size="sm" @click="closeWizard">Close</Button>
          </div>
          <DropdownMenu v-if="signedIn">
            <DropdownMenuTrigger as-child>
              <Button
                variant="outline"
                size="sm"
                :disabled="props.working"
                class="h-9 px-4 text-sm"
              >
                Sign out
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent
              align="end"
              side="top"
              class="z-[120] w-72 rounded-xl border border-border/70 bg-background/75 p-1.5 shadow-2xl ring-1 ring-border/60 backdrop-blur-2xl backdrop-saturate-150"
              style="backdrop-filter: blur(24px) saturate(150%); -webkit-backdrop-filter: blur(24px) saturate(150%);"
            >
              <DropdownMenuItem
                :disabled="props.working || !props.microsoftSignedIn"
                class="h-9 rounded-lg px-3 text-sm"
                @select="emit('sign-out', 'microsoft')"
              >
                Sign out of Microsoft
              </DropdownMenuItem>
              <DropdownMenuSeparator class="mx-0 my-1 bg-border/70" />
              <DropdownMenuItem
                :disabled="props.working"
                class="h-9 rounded-lg px-3 text-sm text-destructive focus:text-destructive"
                @select="emit('sign-out', 'all')"
              >
                Sign out of Atlas + Microsoft
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </Card>
    </div>
  </div>
</template>
