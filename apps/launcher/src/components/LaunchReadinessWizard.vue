<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import { AlertCircle, CheckCircle2, Link2, LoaderCircle, ShieldCheck, Wrench, X } from "lucide-vue-next";
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
import type { LaunchReadinessReport } from "@/types/diagnostics";
import type { LauncherLinkSession } from "@/types/auth";

const props = defineProps<{
  open: boolean;
  readiness: LaunchReadinessReport | null;
  atlasSignedIn: boolean;
  microsoftSignedIn: boolean;
  isSigningIn: boolean;
  linkSession: LauncherLinkSession | null;
  hubUrl: string;
  working: boolean;
  nextActionLabels: Partial<Record<string, string>>;
}>();

const emit = defineEmits<{
  (event: "close"): void;
  (event: "complete"): void;
  (event: "sign-out", scope: "microsoft" | "all"): void;
  (event: "action", key: string): void;
}>();

const LOGIN_KEYS = new Set(["atlasLogin", "microsoftLogin", "accountLink"]);
const checklist = computed(() =>
  (props.readiness?.checklist ?? []).filter((item) => LOGIN_KEYS.has(item.key))
);
const allReady = computed(
  () => checklist.value.length > 0 && checklist.value.every((item) => item.ready)
);
const orderedKeys = ["atlasLogin", "microsoftLogin", "accountLink"] as const;
const copyStatus = ref<string | null>(null);
const autoCompleted = ref(false);
const signedIn = computed(() => props.atlasSignedIn || props.microsoftSignedIn);

const orderedChecklist = computed(() =>
  orderedKeys
    .map((key) => checklist.value.find((item) => item.key === key))
    .filter((item): item is NonNullable<typeof item> => !!item)
);

const activeStep = computed(() => orderedChecklist.value.find((item) => !item.ready) ?? null);
const accountLinkItem = computed(
  () => orderedChecklist.value.find((item) => item.key === "accountLink") ?? null
);

const linkUrl = computed(() => {
  if (!props.linkSession) {
    return null;
  }
  const base = props.hubUrl.replace(/\/$/, "");
  return `${base}/link/launcher?code=${encodeURIComponent(props.linkSession.linkCode)}`;
});

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

const accountLinkStatus = computed(() => {
  const item = accountLinkItem.value;
  if (!item) {
    return null;
  }

  if (item.ready) {
    return null;
  }

  if (props.isSigningIn) {
    return {
      tone: "muted",
      text: "Waiting for browser sign-in to finish..."
    } as const;
  }

  if (props.linkSession) {
    return {
      tone: "warning",
      text: "Waiting for Atlas to confirm this link code."
    } as const;
  }

  return {
    tone: "muted",
    text: "Start this step to generate a link code."
  } as const;
});

watch(
  () => props.open,
  (open) => {
    if (open) {
      autoCompleted.value = false;
    }
  }
);

watch(
  () => allReady.value,
  (ready, wasReady) => {
    if (!props.open || !ready || wasReady || autoCompleted.value) {
      return;
    }
    autoCompleted.value = true;
    emit("complete");
  }
);
</script>

<template>
  <div v-if="props.open" class="fixed inset-0 z-[70] bg-black/55 backdrop-blur-[6px] p-4 md:p-6">
    <div class="mx-auto flex h-full max-w-3xl items-center justify-center">
      <Card class="glass relative w-full max-h-full overflow-hidden bg-background/95">
        <button
          v-if="allReady"
          class="absolute right-6 top-5 inline-flex h-8 w-8 items-center justify-center rounded-md border border-border/70 text-muted-foreground hover:bg-background/60 hover:text-foreground"
          type="button"
          :disabled="props.working"
          aria-label="Close"
          @click="emit('close')"
        >
          <X class="h-4 w-4" />
        </button>
        <CardHeader class="space-y-3 pr-14">
          <CardTitle>Get Ready to Play</CardTitle>
          <CardDescription>
            Sign in and connect your accounts.
          </CardDescription>
        </CardHeader>
        <CardContent class="space-y-4 overflow-y-auto">
          <div v-if="!props.readiness" class="rounded-xl border border-border/60 bg-background/40 p-3 text-sm text-muted-foreground">
            Loading readiness status...
          </div>

          <div v-if="allReady" class="rounded-xl border border-emerald-500/40 bg-emerald-500/10 p-4">
            <div class="flex items-center gap-2 text-sm font-semibold text-emerald-700 dark:text-emerald-300">
              <ShieldCheck class="h-4 w-4" />
              Ready
            </div>
            <p class="mt-1 text-xs text-muted-foreground">
              You can launch now.
            </p>
          </div>

          <div
            v-for="(item, index) in orderedChecklist"
            :key="item.key"
            class="rounded-xl border bg-background/30 p-4"
            :class="item.ready ? 'border-emerald-500/40' : item.key === activeStep?.key ? 'border-amber-500/60' : 'border-border/50'"
          >
            <div class="flex items-start justify-between gap-3">
              <div class="min-w-0">
                <div class="flex items-center gap-2 text-sm font-semibold">
                  <span class="inline-flex h-5 w-5 items-center justify-center rounded-full border border-border/70 text-[11px]">
                    {{ index + 1 }}
                  </span>
                  <CheckCircle2 v-if="item.ready" class="h-4 w-4 text-emerald-500" />
                  <AlertCircle v-else class="h-4 w-4 text-amber-500" />
                  <span>{{ item.label }}</span>
                </div>
                <p v-if="item.detail" class="mt-1 text-xs text-muted-foreground">
                  {{ item.detail }}
                </p>
                <p
                  v-if="!item.ready && item.key === activeStep?.key && props.isSigningIn"
                  class="mt-1 flex items-center gap-1.5 text-xs text-muted-foreground"
                >
                  <LoaderCircle class="h-3.5 w-3.5 animate-spin" />
                  Waiting for sign-in completion...
                </p>
              </div>
              <Button
                v-if="!item.ready && item.key === activeStep?.key && props.nextActionLabels[item.key]"
                variant="secondary"
                size="sm"
                :disabled="props.working"
                @click="emit('action', item.key)"
              >
                <Wrench class="mr-1 h-3.5 w-3.5" />
                {{ props.nextActionLabels[item.key] }}
              </Button>
            </div>

            <div
              v-if="item.key === 'accountLink' && !item.ready && props.linkSession"
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

            <p
              v-if="item.key === 'accountLink' && accountLinkStatus"
              class="mt-2 text-xs"
              :class="accountLinkStatus.tone === 'success'
                ? 'text-emerald-700 dark:text-emerald-300'
                : accountLinkStatus.tone === 'warning'
                  ? 'text-amber-700 dark:text-amber-300'
                  : 'text-muted-foreground'"
            >
              {{ accountLinkStatus.text }}
            </p>
          </div>
        </CardContent>
        <div v-if="signedIn" class="border-t border-border/50 px-6 py-3">
          <DropdownMenu>
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
              align="start"
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
