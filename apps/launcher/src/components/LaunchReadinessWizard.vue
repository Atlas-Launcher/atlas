<script setup lang="ts">
import { computed, ref } from "vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import { AlertCircle, CheckCircle2, Link2, LoaderCircle, ShieldCheck, Wrench } from "lucide-vue-next";
import Button from "./ui/button/Button.vue";
import Card from "./ui/card/Card.vue";
import CardContent from "./ui/card/CardContent.vue";
import CardDescription from "./ui/card/CardDescription.vue";
import CardFooter from "./ui/card/CardFooter.vue";
import CardHeader from "./ui/card/CardHeader.vue";
import CardTitle from "./ui/card/CardTitle.vue";
import type { LaunchReadinessReport } from "@/types/diagnostics";
import type { LauncherLinkSession } from "@/types/auth";

const props = defineProps<{
  open: boolean;
  readiness: LaunchReadinessReport | null;
  isSigningIn: boolean;
  linkSession: LauncherLinkSession | null;
  hubUrl: string;
  working: boolean;
  nextActionLabels: Partial<Record<string, string>>;
}>();

const emit = defineEmits<{
  (event: "close"): void;
  (event: "complete"): void;
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

const orderedChecklist = computed(() =>
  orderedKeys
    .map((key) => checklist.value.find((item) => item.key === key))
    .filter((item): item is NonNullable<typeof item> => !!item)
);

const activeStep = computed(() => orderedChecklist.value.find((item) => !item.ready) ?? null);

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
</script>

<template>
  <div v-if="props.open" class="fixed inset-0 z-[70] bg-black/45 backdrop-blur-[2px] p-4 md:p-6">
    <div class="mx-auto flex h-full max-w-3xl items-center justify-center">
      <Card class="glass w-full max-h-full overflow-hidden">
        <CardHeader>
          <CardTitle>Get Ready to Play</CardTitle>
          <CardDescription>
            Sign in and link your accounts in a few quick steps.
          </CardDescription>
        </CardHeader>
        <CardContent class="space-y-4 overflow-y-auto">
          <div v-if="!props.readiness" class="rounded-xl border border-border/60 bg-background/40 p-3 text-sm text-muted-foreground">
            Loading readiness status...
          </div>

          <div v-if="allReady" class="rounded-xl border border-emerald-500/40 bg-emerald-500/10 p-4">
            <div class="flex items-center gap-2 text-sm font-semibold text-emerald-700 dark:text-emerald-300">
              <ShieldCheck class="h-4 w-4" />
              You are ready
            </div>
            <p class="mt-1 text-xs text-muted-foreground">
              Sign-in and account linking are complete.
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
