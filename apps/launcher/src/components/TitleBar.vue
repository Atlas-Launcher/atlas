<script setup lang="ts">
import { getCurrentWindow } from "@tauri-apps/api/window";
import { onMounted, ref, computed } from "vue";
import { X, Minus, Check, Square, Copy, ChevronDown, Globe, LogIn, LogOut } from "lucide-vue-next";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger
} from "@/components/ui/dropdown-menu";
import type { AtlasProfile, Profile } from "@/types/auth";

const props = defineProps<{
  profile: Profile | null;
  atlasProfile: AtlasProfile | null;
}>();

const emit = defineEmits<{
  (event: "sign-in-microsoft"): void;
  (event: "sign-out-microsoft"): void;
  (event: "sign-in-atlas"): void;
  (event: "sign-out-atlas"): void;
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

function atlasIdentity(profile: AtlasProfile): string {
  return profile.name?.trim() || profile.email?.trim() || profile.id;
}

const msStatus = computed(() => {
  return props.profile ? props.profile.name : "Signed out";
});

const atlasStatus = computed(() => {
  return props.atlasProfile ? atlasIdentity(props.atlasProfile) : "Signed out";
});

const atlasSignedIn = computed(() => !!props.atlasProfile);
const mojangSignedIn = computed(() => !!props.profile);

const statusText = computed(() => {
  if (mojangSignedIn.value && atlasSignedIn.value) return "Signed In";
  if (mojangSignedIn.value || atlasSignedIn.value) return "Sign In Incomplete";
  return "Not Signed In";
});

const statusDotClass = computed(() => {
  if (mojangSignedIn.value && atlasSignedIn.value) {
    return "bg-emerald-500 shadow-[0_0_10px_rgba(16,185,129,0.5)]";
  }
  if (mojangSignedIn.value || atlasSignedIn.value) {
    return "bg-yellow-500 shadow-[0_0_10px_rgba(16,185,129,0.5)]";
  }
  return "bg-red-500 shadow-[0_0_10px_rgba(16,185,129,0.5)]";
});
</script>

<template>
  <div 
    class="fixed top-0 left-0 right-0 h-11 z-50 flex items-center justify-between select-none px-3 transition-colors"
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

    <!-- Right Section: Auth & Controls -->
    <div class="flex items-center gap-2.5 h-full pr-0.5" data-tauri-drag-region>
      <DropdownMenu>
        <DropdownMenuTrigger class="glass group flex items-center h-8 px-4 rounded-2xl hover:bg-foreground/[0.08] hover:border-foreground/[0.18] transition-all duration-300">
          <span 
            class="w-2 h-2 rounded-full mr-2.5 transition-all duration-300 group-hover:scale-110" 
            :class="statusDotClass"
          ></span>
          <div class="text-xs tracking-tight transition-colors duration-300">
            {{ statusText }}
          </div>
          <ChevronDown class="ml-2 h-3 w-3 opacity-20 group-hover:opacity-60 transition-all duration-300" />
        </DropdownMenuTrigger>
        <DropdownMenuContent class="glass w-64 p-2 select-none rounded-2xl" align="end">
          <DropdownMenuLabel class="px-2.5 py-1.5 text-xs font-black uppercase tracking-[0.15em] text-foreground/30">
            Atlas
            <Check v-if="atlasSignedIn" class="inline h-3.5 w-3.5 ml-1" />
            <X v-else class="inline h-3.5 w-3.5 ml-1" />
          </DropdownMenuLabel>
          <div class="px-2.5 pb-2 text-sm font-bold tracking-tight">
            {{ props.atlasProfile ? atlasIdentity(props.atlasProfile) : "Not signed in" }}
          </div>
          <DropdownMenuItem
              v-if="!props.atlasProfile"
              class="ml-2 gap-2 py-2 rounded-xl text-[11px] font-bold bg-foreground/[0.04] hover:bg-foreground/[0.1] transition-colors"
              @select="emit('sign-in-atlas')"
          >
            <Globe class="h-3.5 w-3.5 opacity-80" /> Connect
          </DropdownMenuItem>
          <DropdownMenuItem
              v-else
              class="ml-2 gap-2 py-2 rounded-xl text-[11px] font-bold text-destructive hover:bg-destructive/10"
              @select="emit('sign-out-atlas')"
          >
            <LogOut class="h-3.5 w-3.5" /> Disconnect
          </DropdownMenuItem>
          <DropdownMenuSeparator class="bg-foreground/5 mx-1" />
          <DropdownMenuLabel class="px-2.5 py-1.5 text-xs font-black uppercase tracking-[0.15em] text-foreground/30">Mojang
            <Check v-if="mojangSignedIn" class="inline h-3.5 w-3.5 ml-1" />
            <X v-else class="inline h-3.5 w-3.5 ml-1" /> </DropdownMenuLabel>
          <div class="px-2.5 pb-2 text-sm font-bold tracking-tight">
            {{ props.profile ? props.profile.name : "Not signed in" }}
          </div>
          <DropdownMenuItem
              v-if="!props.profile"
              class="ml-2 gap-2 py-2 rounded-xl text-[11px] font-bold bg-foreground/[0.04] hover:bg-foreground/[0.1] transition-colors"
              @select="emit('sign-in-microsoft')"
          >
            <LogIn class="h-3.5 w-3.5 opacity-80" /> Microsoft
          </DropdownMenuItem>
          <DropdownMenuItem
              v-else
              class="ml-2 gap-2 py-2 rounded-xl text-[11px] font-bold text-destructive hover:bg-destructive/10"
              @select="emit('sign-out-microsoft')"
          >
            <LogOut class="h-3.5 w-3.5" /> Disconnect
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>

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
