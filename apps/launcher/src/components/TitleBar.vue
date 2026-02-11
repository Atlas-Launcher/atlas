<script setup lang="ts">
import { getCurrentWindow } from "@tauri-apps/api/window";
import { onMounted, ref, computed } from "vue";
import { X, Minus, Square, Copy } from "lucide-vue-next";
import type { AtlasProfile, Profile } from "@/types/auth";

const props = defineProps<{
  profile: Profile | null;
  atlasProfile: AtlasProfile | null;
  isSigningIn: boolean;
}>();

const emit = defineEmits<{
  (event: "open-readiness-wizard"): void;
}>();

const isMac = ref(false);
const isMaximized = ref(false);

onMounted(async () => {
  isMac.value = navigator.userAgent.includes("Mac");
  const win = getCurrentWindow();
  isMaximized.value = await win.isMaximized();
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
const hasLinkedMojang = computed(() => !!props.atlasProfile?.mojang_uuid);

function normalizeUuid(value?: string | null) {
  return (value ?? "").trim().toLowerCase().replace(/-/g, "");
}

const isLaunchReady = computed(() => {
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

const needsSetup = computed(() => needsLinking.value || needsLinkCompletion.value);

const statusText = computed(() => {
  if (props.isSigningIn) return "Signing in";
  if (!atlasSignedIn.value) return "Sign in to Atlas";
  if (!mojangSignedIn.value) return "Sign in with Microsoft";
  if (needsSetup.value) return "Get ready";
  if (!isLaunchReady.value) return "Get ready";
  return "Ready";
});

const statusDotClass = computed(() => {
  if (isLaunchReady.value) {
    return "bg-emerald-500 shadow-[0_0_10px_rgba(16,185,129,0.5)]";
  }
  if (mojangSignedIn.value || atlasSignedIn.value) {
    return "bg-yellow-500 shadow-[0_0_10px_rgba(16,185,129,0.5)]";
  }
  return "bg-red-500 shadow-[0_0_10px_rgba(16,185,129,0.5)]";
});
const needsLinking = computed(() => !!props.atlasProfile && !hasLinkedMojang.value);
const needsLinkCompletion = computed(() => {
  if (!props.atlasProfile) {
    return false;
  }
  const normalizeUuid = (value?: string | null) =>
    (value ?? "").trim().toLowerCase().replace(/-/g, "");
  const launcherUuid = normalizeUuid(props.profile?.id);
  const atlasUuid = normalizeUuid(props.atlasProfile.mojang_uuid);
  if (!launcherUuid) {
    return false;
  }
  if (!atlasUuid) {
    return true;
  }
  return atlasUuid !== launcherUuid;
});
</script>

<template>
  <div 
    class="fixed top-0 left-0 right-0 h-11 z-50 flex items-center justify-between select-none px-3 transition-colors"
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

    <!-- Right Section: Launch Readiness & Controls -->
    <div class="flex items-center gap-2.5 h-full pr-0.5">
      <button
        class="glass group flex items-center h-8 px-4 rounded-2xl hover:bg-foreground/[0.08] hover:border-foreground/[0.18] transition-all duration-300"
        :class="{ 'bg-amber-500/10 border-amber-500/30': needsSetup }"
        @click.stop="emit('open-readiness-wizard')"
      >
        <span
          class="w-2 h-2 rounded-full mr-2.5 transition-all duration-300 group-hover:scale-110"
          :class="needsSetup ? 'bg-amber-500 shadow-[0_0_10px_rgba(245,158,11,0.5)]' : statusDotClass"
        ></span>
        <div class="text-xs tracking-tight transition-colors duration-300" :class="{ 'text-amber-500 font-bold': needsSetup }">
          {{ needsSetup ? "Get ready" : statusText }}
        </div>
      </button>

      <div v-if="!isMac" class="glass flex items-center h-8 rounded-full px-1 gap-0.5">
        <button class="h-6 w-8 flex items-center justify-center rounded-full hover:bg-foreground/[0.05] active:bg-foreground/[0.1] transition-colors" @click="minimize" tabindex="-1">
          <Minus class="w-3.5 h-3.5 translate-y-[0.5px] opacity-40 hover:opacity-100 transition-opacity" />
        </button>
        <button class="h-6 w-8 flex items-center justify-center rounded-full hover:bg-foreground/[0.05] active:bg-foreground/[0.1] transition-colors" @click="toggleMaximize" tabindex="-1">
          <Square v-if="!isMaximized" class="w-3 h-3 translate-y-[0.5px] opacity-40 hover:opacity-100 transition-opacity" />
          <Copy v-else class="w-3 h-3 translate-y-[0.5px] opacity-40 hover:opacity-100 transition-opacity" />
        </button>
        <button class="h-6 w-8 flex items-center justify-center rounded-full hover:bg-destructive hover:text-white transition-colors" @click="closeApp" tabindex="-1">
          <X class="w-3.5 h-3.5 translate-y-[0.5px] opacity-40 hover:opacity-100 transition-opacity" />
        </button>
      </div>
    </div>
  </div>
</template>
