<script setup lang="ts">
import { computed, ref } from "vue";
import Button from "./ui/button/Button.vue";
import Card from "./ui/card/Card.vue";
import CardHeader from "./ui/card/CardHeader.vue";
import CardTitle from "./ui/card/CardTitle.vue";
import CardDescription from "./ui/card/CardDescription.vue";
import CardContent from "./ui/card/CardContent.vue";
import MemorySelector from "./MemorySelector.vue";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "./ui/tabs";

const props = withDefaults(defineProps<{
  settingsDefaultMemoryMb: number;
  settingsMemoryMaxMb?: number | null;
  settingsRecommendedMemoryMb?: number | null;
  settingsSystemMemoryMb?: number | null;
  settingsDefaultJvmArgs: string;
  settingsThemeMode: "light" | "dark" | "system";
  working: boolean;
  updaterBusy?: boolean;
  updaterStatusText?: string;
  updaterUpdateVersion?: string | null;
  updaterInstallComplete?: boolean;
}>(), {
  settingsMemoryMaxMb: null,
  settingsRecommendedMemoryMb: null,
  settingsSystemMemoryMb: null,
  updaterBusy: false,
  updaterStatusText: "",
  updaterUpdateVersion: null,
  updaterInstallComplete: false
});

const emit = defineEmits<{
  (event: "update:settingsDefaultMemoryMb", value: number): void;
  (event: "update:settingsDefaultJvmArgs", value: string): void;
  (event: "update:settingsThemeMode", value: "light" | "dark" | "system"): void;
  (event: "check-updates"): void;
  (event: "open-readiness-wizard"): void;
}>();

const settingsTab = ref<"runtime" | "appearance" | "advanced">("runtime");

function updateDefaultMemory(value: number) {
  emit("update:settingsDefaultMemoryMb", value);
}

function updateDefaultJvmArgs(event: Event) {
  const target = event.target as HTMLTextAreaElement | null;
  emit("update:settingsDefaultJvmArgs", target?.value ?? "");
}

function updateThemeMode(value: string) {
  emit("update:settingsThemeMode", value as "light" | "dark" | "system");
}

const updaterPrimaryText = computed(() => {
  if (props.updaterInstallComplete) {
    return "Update is ready to apply.";
  }
  if (props.updaterUpdateVersion) {
    return `Version ${props.updaterUpdateVersion} is available.`;
  }
  if (props.updaterBusy) {
    return "Checking for updates...";
  }
  return "Check for updates when you need one.";
});

const updaterDetailText = computed(() => {
  const status = (props.updaterStatusText ?? "").trim();
  if (!status) {
    return null;
  }
  if (props.updaterUpdateVersion && status === `Update ${props.updaterUpdateVersion} is available.`) {
    return null;
  }
  if (props.updaterInstallComplete && status === "Update installed. Relaunch to finish.") {
    return null;
  }
  return status;
});
</script>

<template>
  <Card class="glass h-full min-h-0 rounded-2xl flex flex-col">
    <CardHeader class="pt-7">
      <CardTitle>Settings</CardTitle>
      <CardDescription>Set launcher defaults and account options.</CardDescription>
    </CardHeader>
    <CardContent class="flex-1 min-h-0 overflow-y-auto space-y-4 pr-3 pb-5 pt-1 [scrollbar-gutter:stable]">
      <Tabs v-model="settingsTab" class="space-y-4">
        <TabsList class="grid w-full grid-cols-3">
          <TabsTrigger value="runtime">Runtime</TabsTrigger>
          <TabsTrigger value="appearance">Appearance</TabsTrigger>
          <TabsTrigger value="advanced">Advanced</TabsTrigger>
        </TabsList>

        <TabsContent value="runtime" class="space-y-4">
          <MemorySelector
            title="Default memory"
            :model-value="props.settingsDefaultMemoryMb"
            :max-mb="props.settingsMemoryMaxMb"
            :recommended-mb="props.settingsRecommendedMemoryMb"
            :system-memory-mb="props.settingsSystemMemoryMb"
            :working="props.working"
            :show-limits-copy="true"
            :show-recommended="true"
            @update:modelValue="updateDefaultMemory"
          />

          <div class="space-y-2">
            <label class="text-xs uppercase tracking-widest text-muted-foreground">
              Default JVM options
            </label>
            <textarea
              class="w-full rounded-xl border border-input bg-background px-3 py-2 text-sm text-foreground shadow-sm outline-none transition focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
              rows="4"
              :value="props.settingsDefaultJvmArgs"
              placeholder="-XX:+UseG1GC -XX:+UnlockExperimentalVMOptions"
              @input="updateDefaultJvmArgs"
            />
            <p class="text-xs text-muted-foreground">
              Used when a profile does not define its own runtime overrides.
            </p>
          </div>
        </TabsContent>

        <TabsContent value="appearance" class="space-y-4">
          <div class="space-y-2">
            <label class="text-xs uppercase tracking-widest text-muted-foreground">
              Theme
            </label>
            <div class="grid grid-cols-3 gap-2">
              <Button
                variant="outline"
                :class="{ 'border-primary ring-1 ring-primary': props.settingsThemeMode === 'light' }"
                @click="updateThemeMode('light')"
              >
                Light
              </Button>
              <Button
                variant="outline"
                :class="{ 'border-primary ring-1 ring-primary': props.settingsThemeMode === 'dark' }"
                @click="updateThemeMode('dark')"
              >
                Dark
              </Button>
              <Button
                variant="outline"
                :class="{ 'border-primary ring-1 ring-primary': props.settingsThemeMode === 'system' }"
                @click="updateThemeMode('system')"
              >
                System
              </Button>
            </div>
            <p class="text-xs text-muted-foreground">
              Choose your preferred appearance or match system settings.
            </p>
          </div>
        </TabsContent>

        <TabsContent value="advanced" class="space-y-4">
          <div class="space-y-2 rounded-xl border border-border bg-muted/20 p-3">
            <div class="flex items-center justify-between gap-3">
              <div class="space-y-1">
                <p class="text-xs uppercase tracking-widest text-muted-foreground">App updates</p>
                <p class="text-sm">{{ updaterPrimaryText }}</p>
              </div>
              <Button
                size="sm"
                variant="outline"
                :disabled="props.updaterBusy"
                @click="emit('check-updates')"
              >
                {{ props.updaterBusy ? "Checking..." : "Check now" }}
              </Button>
            </div>
            <p v-if="updaterDetailText" class="text-xs text-muted-foreground">{{ updaterDetailText }}</p>
          </div>
        </TabsContent>
      </Tabs>
    </CardContent>
  </Card>
</template>
