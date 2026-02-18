<script setup lang="ts">
import { computed, ref } from "vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import Button from "./ui/button/Button.vue";
import Card from "./ui/card/Card.vue";
import CardHeader from "./ui/card/CardHeader.vue";
import CardTitle from "./ui/card/CardTitle.vue";
import CardDescription from "./ui/card/CardDescription.vue";
import CardContent from "./ui/card/CardContent.vue";
import CardFooter from "./ui/card/CardFooter.vue";
import type { AtlasProfile, LauncherLinkSession, Profile } from "@/types/auth";

const props = defineProps<{
  linkSession: LauncherLinkSession | null;
  atlasProfile: AtlasProfile | null;
  profile: Profile | null;
  hubUrl: string;
  working: boolean;
}>();

const copyStatus = ref<string | null>(null);
function normalizeUuid(value?: string | null) {
  const lower = (value ?? "").trim().toLowerCase();
  const candidate = lower.startsWith("urn:uuid:") ? lower.slice("urn:uuid:".length) : lower;
  const hex = candidate.replace(/[^0-9a-f]/g, "");
  return hex.length === 32 ? hex : "";
}

const needsLinking = computed(
  () => !!props.atlasProfile && !normalizeUuid(props.atlasProfile?.mojang_uuid)
);
const linkUrl = computed(() => {
  if (!props.linkSession) return null;
  const base = props.hubUrl.replace(/\/$/, "");
  return `${base}/link/launcher?code=${encodeURIComponent(props.linkSession.linkCode)}`;
});

async function copyCode() {
  if (!props.linkSession) return;
  try {
    await navigator.clipboard.writeText(props.linkSession.linkCode);
    copyStatus.value = "Copied.";
  } catch {
    copyStatus.value = "Copy failed.";
  }
}

async function openLink() {
  if (!linkUrl.value) return;
  await openUrl(linkUrl.value);
}
</script>

<template>
  <Card v-if="needsLinking" class="glass">
    <CardHeader>
      <CardTitle>Link your launcher</CardTitle>
      <CardDescription>
        Connect your Minecraft profile to Atlas using a one-time link code.
      </CardDescription>
    </CardHeader>
    <CardContent class="space-y-4">
      <div class="rounded-2xl border border-border/60 bg-card/70 px-4 py-3 text-xs text-muted-foreground">
        <div class="text-xs uppercase tracking-widest text-muted-foreground">Status</div>
        <div class="mt-2 text-sm text-foreground">
          {{ props.profile ? "Minecraft signed in" : "Step 1: Sign in with Microsoft." }}
        </div>
      </div>

      <div v-if="props.linkSession" class="rounded-2xl border border-border/60 bg-card/70 px-4 py-3">
        <div class="text-xs uppercase tracking-widest text-muted-foreground">Link code</div>
        <div class="mt-2 text-2xl font-semibold tracking-[0.2em] text-foreground">
          {{ props.linkSession.linkCode }}
        </div>
        <div class="mt-2 text-xs text-muted-foreground">
          Expires at {{ props.linkSession.expiresAt }}
        </div>
        <div v-if="copyStatus" class="mt-2 text-xs text-muted-foreground">
          {{ copyStatus }}
        </div>
      </div>
    </CardContent>
    <CardFooter class="flex flex-wrap gap-2">
      <Button variant="outline" :disabled="props.working" @click="copyCode" v-if="props.linkSession">
        Copy code
      </Button>
      <Button variant="outline" :disabled="props.working" @click="openLink" v-if="props.linkSession">
        Open link page
      </Button>
    </CardFooter>
  </Card>
</template>
