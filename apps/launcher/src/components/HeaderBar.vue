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
import { ChevronDown, LogIn, LogOut, Settings as SettingsIcon, User } from "lucide-vue-next";
import type { Profile } from "@/types/auth";

type TabKey = "library" | "settings";

const props = defineProps<{
  activeTab: TabKey;
  profile: Profile | null;
}>();

const emit = defineEmits<{
  (event: "sign-in"): void;
  (event: "account-settings"): void;
  (event: "sign-out"): void;
}>();

const title = computed(() => (props.activeTab === "library" ? "Library" : "Settings"));
const description = computed(() =>
  props.activeTab === "library"
    ? "Pick a profile and launch when you are ready."
    : "Sign in and tweak advanced preferences."
);
</script>

<template>
  <header class="flex flex-wrap items-center justify-between gap-4">
    <div>
      <p class="text-xs uppercase tracking-[0.35em] text-muted-foreground">Atlas Launcher</p>
    </div>
    <DropdownMenu>
      <DropdownMenuTrigger class="status-pill">
        <span class="status-dot" :class="profile ? 'status-dot--online' : 'status-dot--offline'"></span>
        <span>
          {{ profile ? `Signed in as ${profile.name}` : "Sign in with Microsoft" }}
        </span>
        <ChevronDown class="h-4 w-4 text-muted-foreground" />
      </DropdownMenuTrigger>
      <DropdownMenuContent class="w-64 backdrop-blur-3xl" align="end">
        <DropdownMenuLabel>Microsoft account</DropdownMenuLabel>
        <div class="px-2 pb-2 text-sm text-muted-foreground">
          {{ profile ? profile.name : "Not signed in" }}
        </div>
        <DropdownMenuSeparator />
        <DropdownMenuItem v-if="!profile" @select="emit('sign-in')">
          <LogIn class="h-4 w-4" />
          Sign in
        </DropdownMenuItem>
        <DropdownMenuItem v-if="profile" @select="emit('account-settings')">
          <User class="h-4 w-4" />
          Account settings
        </DropdownMenuItem>
        <DropdownMenuItem v-if="profile" @select="emit('sign-out')">
          <LogOut class="h-4 w-4" />
          Sign out
        </DropdownMenuItem>
        <DropdownMenuItem v-else @select="emit('account-settings')">
          <SettingsIcon class="h-4 w-4" />
          More options
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  </header>
</template>
