<script setup lang="ts">
import { ref, withDefaults } from "vue";
import { ShieldAlert } from "lucide-vue-next";
import Button from "./ui/button/Button.vue";
import Card from "./ui/card/Card.vue";
import CardHeader from "./ui/card/CardHeader.vue";
import CardTitle from "./ui/card/CardTitle.vue";
import CardDescription from "./ui/card/CardDescription.vue";
import CardContent from "./ui/card/CardContent.vue";
import CardFooter from "./ui/card/CardFooter.vue";
import Input from "./ui/input/Input.vue";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "./ui/tabs";

const props = withDefaults(defineProps<{
  settingsClientId: string;
  settingsAtlasHubUrl: string;
  settingsDefaultMemoryMb: number;
  settingsDefaultJvmArgs: string;
  settingsThemeMode: "light" | "dark" | "system";
  working: boolean;
  updaterBusy?: boolean;
  updaterStatusText?: string;
  updaterUpdateVersion?: string | null;
  updaterInstallComplete?: boolean;
}>(), {
  updaterBusy: false,
  updaterStatusText: "",
  updaterUpdateVersion: null,
  updaterInstallComplete: false
});

const emit = defineEmits<{
  (event: "update:settingsClientId", value: string): void;
  (event: "update:settingsAtlasHubUrl", value: string): void;
  (event: "update:settingsDefaultMemoryMb", value: number): void;
  (event: "update:settingsDefaultJvmArgs", value: string): void;
  (event: "update:settingsThemeMode", value: "light" | "dark" | "system"): void;
  (event: "save-settings"): void;
  (event: "check-updates"): void;
  (event: "open-readiness-wizard"): void;
  (event: "open-troubleshooter"): void;
}>();

const settingsTab = ref<"runtime" | "appearance" | "advanced">("runtime");

function updateClientId(value: string | number) {
  emit("update:settingsClientId", String(value ?? ""));
}

function updateAtlasHubUrl(value: string | number) {
  emit("update:settingsAtlasHubUrl", String(value ?? ""));
}

function updateDefaultMemory(value: string | number) {
  emit("update:settingsDefaultMemoryMb", Number(value));
}

function updateDefaultJvmArgs(event: Event) {
  const target = event.target as HTMLTextAreaElement | null;
  emit("update:settingsDefaultJvmArgs", target?.value ?? "");
}

function updateThemeMode(value: string) {
  emit("update:settingsThemeMode", value as "light" | "dark" | "system");
}
</script>

<template>
  <Card class="glass">
    <CardHeader>
      <CardTitle>Settings</CardTitle>
      <CardDescription>Manage launcher defaults and sign-in options.</CardDescription>
    </CardHeader>
    <CardContent class="space-y-4">
      <Tabs v-model="settingsTab" class="space-y-4">
        <TabsList class="grid w-full grid-cols-3">
          <TabsTrigger value="runtime">Runtime</TabsTrigger>
          <TabsTrigger value="appearance">Appearance</TabsTrigger>
          <TabsTrigger value="advanced">Advanced</TabsTrigger>
        </TabsList>

        <TabsContent value="runtime" class="space-y-4">
          <div class="space-y-2">
            <label class="text-xs uppercase tracking-widest text-muted-foreground">
              Default Java memory (MB)
            </label>
            <Input
              type="number"
              min="1024"
              :model-value="props.settingsDefaultMemoryMb"
              @update:modelValue="updateDefaultMemory"
            />
          </div>
          <div class="space-y-2">
            <label class="text-xs uppercase tracking-widest text-muted-foreground">
              Default JVM launch options
            </label>
            <textarea
              class="w-full rounded-xl border border-input bg-background px-3 py-2 text-sm text-foreground shadow-sm outline-none transition focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
              rows="4"
              :value="props.settingsDefaultJvmArgs"
              placeholder="-XX:+UseG1GC -XX:+UnlockExperimentalVMOptions"
              @input="updateDefaultJvmArgs"
            />
            <p class="text-xs text-muted-foreground">
              Applied when a profile does not override runtime settings.
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
                <p class="text-sm">
                  <template v-if="props.updaterInstallComplete">
                    Update installed.
                    <span class="text-primary font-medium">Restart is required.</span>
                  </template>
                  <template v-else-if="props.updaterUpdateVersion">
                    Update {{ props.updaterUpdateVersion }} is available.
                  </template>
                  <template v-else>
                    Check for launcher updates manually.
                  </template>
                </p>
              </div>
              <Button
                size="sm"
                variant="outline"
                :disabled="props.updaterBusy"
                @click="emit('check-updates')"
              >
                {{ props.updaterBusy ? "Checking..." : "Check for updates" }}
              </Button>
            </div>
            <p class="text-xs text-muted-foreground">{{ props.updaterStatusText }}</p>
          </div>

          <div class="space-y-2">
            <label class="text-xs uppercase tracking-widest text-muted-foreground">
              Microsoft Client ID
            </label>
            <Input
              :model-value="props.settingsClientId"
              placeholder="Leave empty to use the default ID"
              @update:modelValue="updateClientId"
            />
          </div>
          <div class="space-y-2">
            <label class="text-xs uppercase tracking-widest text-muted-foreground">
              Atlas Hub URL
            </label>
            <Input
              :model-value="props.settingsAtlasHubUrl"
              placeholder="https://atlas.nathanm.org"
              @update:modelValue="updateAtlasHubUrl"
            />
          </div>
          <div class="text-xs text-muted-foreground">
            Sign out and sign back in after changing auth settings.
          </div>
        </TabsContent>
      </Tabs>
    </CardContent>
    <CardFooter>
      <div class="flex w-full items-center justify-between gap-3">
        <div class="flex items-center gap-2">
          <Button :disabled="props.working" variant="outline" @click="emit('open-readiness-wizard')">
            <ShieldAlert class="mr-1 h-4 w-4" />
            Get ready to play
          </Button>
          <Button :disabled="props.working" variant="outline" @click="emit('open-troubleshooter')">
            Open troubleshooter
          </Button>
        </div>
        <Button :disabled="props.working" variant="secondary" @click="emit('save-settings')">Save settings</Button>
      </div>
    </CardFooter>
  </Card>
</template>
