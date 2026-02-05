<script setup lang="ts">
import { computed, ref } from "vue";
import Button from "./ui/button/Button.vue";
import Card from "./ui/card/Card.vue";
import CardHeader from "./ui/card/CardHeader.vue";
import CardTitle from "./ui/card/CardTitle.vue";
import CardDescription from "./ui/card/CardDescription.vue";
import CardContent from "./ui/card/CardContent.vue";
import Input from "./ui/input/Input.vue";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./ui/tabs";
import { Label } from "@/components/ui/label";
import type { InstanceConfig } from "@/types/settings";
import { pruneHomePath } from "@/lib/utils";

const props = defineProps<{
  instance: InstanceConfig | null;
  instancesCount: number;
  defaultMemoryMb: number;
  defaultJvmArgs: string;
  working: boolean;
  managedByAtlas?: boolean;
}>();

const emit = defineEmits<{
  (event: "duplicate", id: string): void;
  (event: "remove", id: string): void;
  (event: "update", payload: { id: string; patch: Partial<InstanceConfig> }): void;
}>();

const optionsTab = ref<"profile" | "runtime">("profile");

const displayGameDir = computed(() => {
  if (!props.instance) return "";
  return pruneHomePath(props.instance.gameDir) ?? "Managed by the launcher";
});

const hasRuntimeOverrides = computed(() => {
  const instance = props.instance;
  if (!instance) {
    return false;
  }
  return (
    instance.memoryMb != null ||
    Boolean((instance.jvmArgs ?? "").trim()) ||
    Boolean((instance.javaPath ?? "").trim())
  );
});

function copyGameDir() {
  const dir = props.instance?.gameDir;
  if (!dir || !navigator.clipboard) return;
  navigator.clipboard.writeText(dir).catch(() => {});
}

function patchInstance(patch: Partial<InstanceConfig>) {
  if (!props.instance) {
    return;
  }
  emit("update", { id: props.instance.id, patch });
}

function updateField(key: keyof InstanceConfig, value: string | number | null) {
  patchInstance({ [key]: value } as Partial<InstanceConfig>);
}

function enableRuntimeOverrides() {
  if (!props.instance) {
    return;
  }
  patchInstance({
    memoryMb: props.instance.memoryMb ?? props.defaultMemoryMb,
    jvmArgs: props.instance.jvmArgs ?? (props.defaultJvmArgs.trim() || null)
  });
}

function clearRuntimeOverrides() {
  patchInstance({
    memoryMb: null,
    jvmArgs: null,
    javaPath: ""
  });
}

function updateMemory(value: string | number) {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) {
    updateField("memoryMb", null);
    return;
  }
  updateField("memoryMb", Math.max(1024, Math.round(parsed)));
}

function updateJvmArgs(event: Event) {
  const target = event.target as HTMLTextAreaElement | null;
  const value = (target?.value ?? "").trim();
  updateField("jvmArgs", value || null);
}
</script>

<template>
  <Card class="glass">
    <CardHeader>
      <CardTitle>Profile options</CardTitle>
      <CardDescription>Manage this profile and its runtime behavior.</CardDescription>
    </CardHeader>
    <CardContent class="space-y-4">
      <div v-if="!props.instance" class="text-sm text-muted-foreground">
        Select a profile to edit its settings.
      </div>

      <Tabs v-else v-model="optionsTab" class="space-y-4">
        <TabsList class="grid w-full grid-cols-2">
          <TabsTrigger value="profile">Profile</TabsTrigger>
          <TabsTrigger value="runtime">Runtime override</TabsTrigger>
        </TabsList>

        <TabsContent value="profile" class="space-y-4">
          <div
            v-if="props.managedByAtlas"
            class="rounded-xl border border-border/60 bg-card/70 px-4 py-3 text-xs text-muted-foreground"
          >
            Profile identity is managed by Atlas Hub for remote packs.
          </div>
          <div class="space-y-2">
            <Label class="text-xs uppercase tracking-widest text-muted-foreground">Name</Label>
            <Input
              :model-value="props.instance.name"
              :disabled="props.managedByAtlas"
              @update:modelValue="(value) => updateField('name', String(value))"
            />
          </div>
          <div class="space-y-2">
            <Label class="text-xs uppercase tracking-widest text-muted-foreground">
              Data directory
            </Label>
            <div class="flex items-center gap-2 rounded-xl border border-border/60 bg-card/70 px-3 py-2">
              <div
                role="button"
                tabindex="0"
                class="flex-1 truncate cursor-pointer text-sm text-foreground/90"
                :title="props.instance?.gameDir ?? 'Managed by the launcher'"
                @click="copyGameDir"
                @keydown.enter.prevent="copyGameDir"
                @keydown.space.prevent="copyGameDir"
                aria-label="Data directory (click to copy)"
              >
                {{ displayGameDir }}
              </div>
              <Button size="sm" variant="ghost" @click="copyGameDir">Copy</Button>
            </div>
          </div>
          <div v-if="!props.managedByAtlas" class="flex flex-wrap gap-2">
            <Button
              :disabled="props.working"
              size="sm"
              variant="secondary"
              @click="emit('duplicate', props.instance.id)"
            >
              Duplicate profile
            </Button>
            <Button
              :disabled="props.working || props.instancesCount <= 1"
              size="sm"
              variant="destructive"
              @click="emit('remove', props.instance.id)"
            >
              Delete profile
            </Button>
          </div>
        </TabsContent>

        <TabsContent value="runtime" class="space-y-4">
          <div class="rounded-xl border border-border/60 bg-card/70 px-4 py-3 text-sm">
            <div class="text-xs uppercase tracking-widest text-muted-foreground">Default runtime</div>
            <div class="mt-1 font-semibold text-foreground">
              {{ props.defaultMemoryMb }} MB memory
            </div>
            <div class="mt-1 text-xs text-muted-foreground break-all">
              JVM args:
              {{
                props.defaultJvmArgs.trim().length > 0
                  ? props.defaultJvmArgs
                  : "No global JVM launch options configured."
              }}
            </div>
          </div>

          <div class="flex flex-wrap gap-2">
            <Button
              v-if="!hasRuntimeOverrides"
              :disabled="props.working"
              size="sm"
              variant="secondary"
              @click="enableRuntimeOverrides"
            >
              Enable overrides
            </Button>
            <Button
              v-else
              :disabled="props.working"
              size="sm"
              variant="ghost"
              @click="clearRuntimeOverrides"
            >
              Use default runtime settings
            </Button>
          </div>

          <div v-if="hasRuntimeOverrides" class="grid gap-4">
            <div class="space-y-2">
              <Label class="text-xs uppercase tracking-widest text-muted-foreground">
                Memory (MB)
              </Label>
              <Input
                type="number"
                min="1024"
                :model-value="props.instance.memoryMb ?? props.defaultMemoryMb"
                @update:modelValue="updateMemory"
              />
            </div>
            <div class="space-y-2">
              <Label class="text-xs uppercase tracking-widest text-muted-foreground">
                JVM launch options
              </Label>
              <textarea
                class="w-full rounded-xl border border-input bg-background px-3 py-2 text-sm text-foreground shadow-sm outline-none transition focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
                rows="4"
                :value="props.instance.jvmArgs ?? ''"
                placeholder="-XX:+UseG1GC -XX:+UnlockExperimentalVMOptions"
                @input="updateJvmArgs"
              />
            </div>
            <div class="space-y-2">
              <Label class="text-xs uppercase tracking-widest text-muted-foreground">
                Java path (optional)
              </Label>
              <Input
                :model-value="props.instance.javaPath ?? ''"
                placeholder="Leave empty to auto-manage Java"
                @update:modelValue="(value) => updateField('javaPath', String(value))"
              />
            </div>
          </div>

          <p v-else class="text-xs text-muted-foreground">
            This profile currently inherits runtime settings from Launcher settings.
          </p>
        </TabsContent>
      </Tabs>
    </CardContent>
  </Card>
</template>
