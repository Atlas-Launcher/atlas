<script setup lang="ts">
import { computed } from "vue";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger
} from "./ui/dropdown-menu";
import { ChevronDown, Globe, LogIn, LogOut } from "lucide-vue-next";
import type { AtlasProfile, Profile } from "@/types/auth";

type TabKey = "library" | "settings";

const props = defineProps<{
  activeTab: TabKey;
  profile: Profile | null;
  atlasProfile: AtlasProfile | null;
}>();

const emit = defineEmits<{
  (event: "sign-in-microsoft"): void;
  (event: "sign-out-microsoft"): void;
  (event: "sign-in-atlas"): void;
  (event: "sign-out-atlas"): void;
}>();

function atlasIdentity(profile: AtlasProfile): string {
  return profile.name?.trim() || profile.email?.trim() || profile.id;
}

const msStatus = computed(() => {
  return props.profile ? props.profile.name : "Signed out";
});

const atlasStatus = computed(() => {
  return props.atlasProfile ? atlasIdentity(props.atlasProfile) : "Signed out";
});

const anySignedIn = computed(() => Boolean(props.profile || props.atlasProfile));
</script>

<template>
  <header class="flex flex-wrap items-center justify-between gap-4">
    <div>
      <p class="text-xs uppercase tracking-[0.35em] text-muted-foreground">Atlas Launcher</p>
    </div>
    <DropdownMenu>
      <DropdownMenuTrigger class="status-pill status-pill--accounts">
        <span class="status-dot" :class="anySignedIn ? 'status-dot--online' : 'status-dot--offline'"></span>
        <span class="account-pill-text">
          <span class="account-pill-row">
            <span class="account-pill-label">Atlas:</span>
            <span class="truncate">{{ atlasStatus }}</span>
          </span>
          <span class="account-pill-row">
            <span class="account-pill-label">Mojang:</span>
            <span class="truncate">{{ msStatus }}</span>
          </span>
        </span>
        <ChevronDown class="h-[18px] w-[18px] text-muted-foreground" />
      </DropdownMenuTrigger>
      <DropdownMenuContent class="w-[26rem] p-2.5 backdrop-blur-3xl" align="end">
        <DropdownMenuLabel class="px-3 pt-2 pb-1 text-base font-semibold">Atlas Hub account</DropdownMenuLabel>
        <div class="px-3 pb-3 text-base text-muted-foreground">
          {{ props.atlasProfile ? atlasIdentity(props.atlasProfile) : "Not signed in" }}
        </div>
        <DropdownMenuSeparator />
        <DropdownMenuItem v-if="!props.atlasProfile" class="py-2.5 text-base" @select="emit('sign-in-atlas')">
          <Globe class="h-4 w-4" />
          Sign in
        </DropdownMenuItem>
        <DropdownMenuItem v-if="props.atlasProfile" class="py-2.5 text-base" @select="emit('sign-out-atlas')">
          <LogOut class="h-4 w-4" />
          Sign out
        </DropdownMenuItem>
        <DropdownMenuSeparator />
        <DropdownMenuLabel class="px-3 pt-2 pb-1 text-base font-semibold">Microsoft account</DropdownMenuLabel>
        <div class="px-3 pb-3 text-base text-muted-foreground">
          {{ props.profile ? props.profile.name : "Not signed in" }}
        </div>
        <DropdownMenuSeparator />
        <DropdownMenuItem v-if="!props.profile" class="py-2.5 text-base" @select="emit('sign-in-microsoft')">
          <LogIn class="h-4 w-4" />
          Sign in
        </DropdownMenuItem>
        <DropdownMenuItem v-if="props.profile" class="py-2.5 text-base" @select="emit('sign-out-microsoft')">
          <LogOut class="h-4 w-4" />
          Sign out
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  </header>
</template>
