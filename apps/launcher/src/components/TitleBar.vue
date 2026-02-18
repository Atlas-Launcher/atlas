<script setup lang="ts">
import { getCurrentWindow } from "@tauri-apps/api/window";
import { onMounted, onUnmounted, ref, computed, watch } from "vue";
import { X, Minus, Square, Copy, Check } from "lucide-vue-next";
import { CloudAlert } from "lucide-vue-next";
import playerHeadFallbackPng from "@/assets/player-head-fallback.png";
import type { AtlasProfile, Profile } from "@/types/auth";
import type { LaunchReadinessReport } from "@/types/diagnostics";

const props = defineProps<{
  profile: Profile | null;
  atlasProfile: AtlasProfile | null;
  readiness?: LaunchReadinessReport | null;
  isSigningIn: boolean;
  // New prop: when true, we can't connect out even though required vars exist
  cannotConnect?: boolean;
  // New prop: indicates the launch readiness wizard is currently open
  readinessOpen?: boolean;
}>();

const emit = defineEmits<{
  (event: "open-readiness-wizard"): void;
}>();

const isMac = ref(false);
const isMaximized = ref(false);

// Track navigator online/offline state to show "No Internet" when the system loses network
const isOnline = ref(typeof navigator !== "undefined" ? navigator.onLine : true);

onMounted(async () => {
  isMac.value = navigator.userAgent.includes("Mac");
  const win = getCurrentWindow();
  isMaximized.value = await win.isMaximized();

  const updateOnline = () => {
    isOnline.value = navigator.onLine;
  };
  window.addEventListener("online", updateOnline);
  window.addEventListener("offline", updateOnline);

  onUnmounted(() => {
    window.removeEventListener("online", updateOnline);
    window.removeEventListener("offline", updateOnline);
  });
});

async function minimize() {
  await getCurrentWindow().minimize();
}

async function toggleMaximize() {
  const win = getCurrentWindow();
  await win.toggleMaximize();
  isMaximized.value = await win.isMaximized();
}

async function closeApp() {
  await getCurrentWindow().close();
}

const atlasSignedIn = computed(() => !!props.atlasProfile);
const mojangSignedIn = computed(() => !!props.profile);
const atlasReadyState = computed(() => props.readiness?.atlasLoggedIn ?? atlasSignedIn.value);
const microsoftReadyState = computed(
  () => props.readiness?.microsoftLoggedIn ?? mojangSignedIn.value
);
const LOCAL_FALLBACK_HEAD = playerHeadFallbackPng;

function normalizeUuid(value?: string | null) {
  const lower = (value ?? "").trim().toLowerCase();
  const candidate = lower.startsWith("urn:uuid:") ? lower.slice("urn:uuid:".length) : lower;
  const hex = candidate.replace(/[^0-9a-f]/g, "");
  return hex.length === 32 ? hex : "";
}

function sanitizeDisplayName(value?: string | null) {
  const trimmed = (value ?? "").trim();
  return trimmed.length > 0 ? trimmed : null;
}

const localAccountLinkReady = computed(() => {
  if (!props.atlasProfile || !props.profile) {
    return false;
  }
  const atlasUuid = normalizeUuid(props.atlasProfile.mojang_uuid);
  const launcherUuid = normalizeUuid(props.profile.id);
  if (!atlasUuid || !launcherUuid) {
    return false;
  }
  return atlasUuid === launcherUuid;
});

const isLaunchReady = computed(() => {
  if (props.readiness) {
    return (
      props.readiness.atlasLoggedIn &&
      props.readiness.microsoftLoggedIn &&
      props.readiness.accountsLinked
    );
  }
  return localAccountLinkReady.value;
});

const needsSetup = computed(() => !isLaunchReady.value);

const statusText = computed(() => {
  if (props.isSigningIn) return "Signing in";
  if (!atlasReadyState.value) return "Sign in to Atlas";
  if (!microsoftReadyState.value) return "Sign in with Microsoft";
  if (needsSetup.value) return "Finish setup";
  return "Ready";
});

// Combined no-internet indicator: either backend-reported cannotConnect OR navigator offline
const showNoInternet = computed(() => props.cannotConnect || !isOnline.value);

const displayText = computed(() => {
  if (showNoInternet.value) return "No Internet";
  if (isLaunchReady.value) return statusText.value || "Ready";
  return "Needs setup";
});

const iconVariant = computed(() => {
  if (showNoInternet.value) return "cloud";
  if (isLaunchReady.value) return "check";
  return "x";
});

const readyMojangUsername = computed(() => {
  if (!isLaunchReady.value || showNoInternet.value || props.isSigningIn) {
    return null;
  }

  const microsoftUsername = sanitizeDisplayName(props.profile?.name);
  if (microsoftUsername) {
    return microsoftUsername;
  }

  return sanitizeDisplayName(props.atlasProfile?.mojang_username);
});

const readyMojangUuid = computed(() => {
  if (!isLaunchReady.value || showNoInternet.value || props.isSigningIn) {
    return "";
  }
  return normalizeUuid(props.profile?.id) || normalizeUuid(props.atlasProfile?.mojang_uuid);
});

const readyMojangHeadUrl = computed(() =>
  readyMojangUuid.value
    ? `https://mc-heads.net/avatar/${readyMojangUuid.value}/24`
    : LOCAL_FALLBACK_HEAD
);

const readyHeadImageSrc = ref(LOCAL_FALLBACK_HEAD);

watch(
  () => readyMojangHeadUrl.value,
  (value) => {
    readyHeadImageSrc.value = value;
  },
  { immediate: true }
);

function handleReadyHeadError() {
  if (readyHeadImageSrc.value !== LOCAL_FALLBACK_HEAD) {
    readyHeadImageSrc.value = LOCAL_FALLBACK_HEAD;
  }
}

function handleReadinessClick() {
  // When there's no internet, do nothing on click
  if (showNoInternet.value) return;
  emit("open-readiness-wizard");
}
</script>

<template>
  <div 
    class="fixed top-0 left-0 right-0 h-11 z-[100] flex items-center justify-between select-none px-3 transition-colors"
    data-tauri-drag-region
  >
    <!-- Left Section: Compact spacer for traffic lights -->
    <div class="flex items-center h-full" data-tauri-drag-region>
      <div 
        class="glass flex items-center h-8 rounded-2xl transition-all"
        :class="isMac ? 'w-[76px] justify-center' : 'px-4 justify-center'"
        data-tauri-drag-region
      >
        <div v-if="isMac" class="w-[60px]" data-tauri-drag-region></div>
        <span 
          v-if="!isMac"
          class="text-[12px] font-black text-foreground/45 tracking-[0.2em] uppercase pointer-events-none text-center leading-none flex items-center h-full"
        >
          Atlas Launcher
        </span>
      </div>
      <span v-if="isMac" class="ml-8 text-[12px] font-black text-foreground/15 tracking-[0.2em] uppercase pointer-events-none" data-tauri-drag-region>
        Atlas Launcher
      </span>
    </div>

    <!-- Right Section: Launch Readiness & Controls (anchored to right-4 for a consistent gap) -->
    <div class="absolute right-4 top-0 h-11 flex items-center z-[101]" data-tauri-no-drag>
      <div class="flex items-center gap-2.5 h-full pr-0.5">
        <div
          v-if="readyMojangUsername"
          class="glass flex min-w-0 items-center h-8 pl-1.5 pr-3 rounded-2xl border border-foreground/[0.16] bg-background/25 shadow-[0_8px_24px_-16px_rgba(15,23,42,0.8)]"
          data-tauri-no-drag
        >
          <img
            :src="readyHeadImageSrc"
            alt="Minecraft player head"
            class="h-5 w-5 ml-1 rounded-[4px] shrink-0 border border-foreground/25 bg-background/70 object-cover"
            style="image-rendering: pixelated;"
            loading="lazy"
            @error="handleReadyHeadError"
          />
          <span class="ml-2 max-w-[140px] truncate text-[11px] font-medium tracking-[0.01em] leading-none text-white/95">
            {{ readyMojangUsername }}
          </span>
        </div>

        <button
          class="glass group flex items-center h-8 px-4 rounded-2xl hover:bg-foreground/[0.08] hover:border-foreground/[0.18] transition-all duration-300"
          :class="{ 'bg-amber-500/10 border-amber-500/30': needsSetup, 'opacity-80': props.readinessOpen && !showNoInternet, 'cursor-not-allowed opacity-70': showNoInternet }"
          @click.stop="handleReadinessClick"
          data-tauri-no-drag
        >
          <!-- Icon variants: cloud (no internet), check (ready), x (not ready) -->
          <template v-if="iconVariant === 'cloud'">
            <CloudAlert class="h-4 w-4 text-amber-500 mr-2.5" />
          </template>
          <template v-else-if="iconVariant === 'check'">
            <Check class="h-4 w-4 text-emerald-500 mr-2.5" />
          </template>
          <template v-else>
            <X class="h-4 w-4 text-red-500 mr-2.5" />
          </template>

          <span
            class="text-xs tracking-tight transition-colors duration-300"
            :class="{
              'text-emerald-500 font-bold': iconVariant === 'check',
              'text-red-500 font-bold': iconVariant === 'x',
              'text-amber-500 font-bold': iconVariant === 'cloud'
            }"
          >
            {{ displayText }}
          </span>
        </button>

        <div v-if="!isMac" class="glass flex items-center h-8 rounded-full px-1 gap-0.5" data-tauri-no-drag>
          <button class="h-6 w-8 flex items-center justify-center rounded-full hover:bg-foreground/[0.05] active:bg-foreground/[0.1] transition-colors" @click="minimize" tabindex="-1" data-tauri-no-drag>
            <Minus class="w-3.5 h-3.5 translate-y-[0.5px] opacity-40 hover:opacity-100 transition-opacity" />
          </button>
          <button class="h-6 w-8 flex items-center justify-center rounded-full hover:bg-foreground/[0.05] active:bg-foreground/[0.1] transition-colors" @click="toggleMaximize" tabindex="-1" data-tauri-no-drag>
            <Square v-if="!isMaximized" class="w-3 h-3 translate-y-[0.5px] opacity-40 hover:opacity-100 transition-opacity" />
            <Copy v-else class="w-3 h-3 translate-y-[0.5px] opacity-40 hover:opacity-100 transition-opacity" />
          </button>
          <button class="h-6 w-8 flex items-center justify-center rounded-full hover:bg-destructive hover:text-white transition-colors" @click="closeApp" tabindex="-1" data-tauri-no-drag>
            <X class="w-3.5 h-3.5 translate-y-[0.5px] opacity-40 hover:opacity-100 transition-opacity" />
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
