<script setup lang="ts">
import { computed, ref } from "vue";
import InstanceSettingsCard from "./InstanceSettingsCard.vue";
import ModsCard from "./ModsCard.vue";
import VersionsCard from "./VersionsCard.vue";
import Button from "./ui/button/Button.vue";
import Progress from "./ui/progress/Progress.vue";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./ui/tabs";
import { Box } from "lucide-vue-next";
import type { Profile } from "@/types/auth";
import type { FabricLoaderVersion, ModEntry, VersionSummary } from "@/types/library";
import type { InstanceConfig } from "@/types/settings";
import {ChevronLeftIcon} from "lucide-vue-next";

const props = defineProps<{
  instance: InstanceConfig | null;
  profile: Profile | null;
  working: boolean;
  progress: number;
  mods: ModEntry[];
  modsDir: string;
  availableVersions: VersionSummary[];
  latestRelease: string;
  installedVersions: string[];
  fabricLoaderVersions: FabricLoaderVersion[];
  neoforgeLoaderVersions: string[];
  instancesCount: number;
}>();

const emit = defineEmits<{
  (event: "back"): void;
  (event: "launch"): void;
  (event: "update-files"): void;
  (event: "go-to-settings"): void;
  (event: "toggle-mod", payload: { fileName: string; enabled: boolean }): void;
  (event: "delete-mod", fileName: string): void;
  (event: "refresh-mods"): void;
  (event: "open-mods-folder"): void;
  (event: "update-instance", payload: { id: string; patch: Partial<InstanceConfig> }): void;
  (event: "install-version"): void;
  (event: "refresh-versions"): void;
  (event: "duplicate-instance", id: string): void;
  (event: "remove-instance", id: string): void;
}>();

const detailTab = ref<"content" | "setup">("content");

const activeLoaderLabel = computed(() => {
  const instance = props.instance;
  if (!instance) {
    return "";
  }
  const loader = instance.loader?.kind ?? "vanilla";
  const version = instance.version?.trim() || "Latest release";
  return `${loader} · ${version}`;
});
</script>

<template>
  <!-- Fill available height so the tabs area can scroll internally -->
  <section class="flex-1 min-h-0 flex flex-col gap-6 overflow-hidden">
    <div class="rounded-3xl border border-border/60 bg-card/70 p-4">
      <div class="flex flex-wrap items-center gap-4">
        <Button size="icon-sm" variant="ghost" @click="emit('back')"><ChevronLeftIcon /></Button>
        <div class="flex items-center gap-3">
          <div
            class="flex h-14 w-14 items-center justify-center rounded-2xl border border-border/60 bg-muted"
          >
            <Box class="h-6 w-6 text-muted-foreground" />
          </div>
          <div>
            <div class="text-xl font-semibold text-foreground">
              {{ props.instance?.name ?? "Profile" }}
            </div>
            <div class="text-sm text-muted-foreground">{{ activeLoaderLabel }}</div>
          </div>
        </div>
        <div class="ml-auto flex flex-wrap items-center gap-2">
          <Button :disabled="props.working || !props.profile" @click="emit('launch')">
            Play
          </Button>
          <Button :disabled="props.working" variant="secondary" @click="emit('update-files')">
            Update
          </Button>
          <Button variant="ghost" @click="detailTab = 'setup'">Settings</Button>
        </div>
      </div>
      <div class="mt-4 space-y-2">
        <div class="flex items-center justify-between text-xs text-muted-foreground">
          <span>Launch progress</span>
          <span>{{ props.progress }}%</span>
        </div>
        <Progress :model-value="props.progress" />
      </div>
    </div>

    <div
      v-if="!props.profile"
      class="rounded-2xl border border-border/60 bg-card/70 px-4 py-3 text-sm text-muted-foreground"
    >
      Sign in with Microsoft to launch this profile.
      <Button size="sm" variant="ghost" class="ml-2" @click="emit('go-to-settings')">
        Go to sign in
      </Button>
    </div>

    <!-- Tabs container grows and becomes the inner scroll container -->
    <Tabs v-model="detailTab" class="flex-1 min-h-0 flex flex-col overflow-hidden space-y-6">
      <TabsList class="flex flex-wrap justify-start gap-2 bg-transparent p-0 shrink-0">
        <TabsTrigger value="content">Content</TabsTrigger>
        <TabsTrigger value="setup">Setup</TabsTrigger>
        <!-- Logs are available in Settings -->
      </TabsList>
      <TabsContent value="content" class="space-y-6">
        <ModsCard
          :instance="props.instance"
          :mods="props.mods"
          :mods-dir="props.modsDir"
          :working="props.working"
          @toggle="emit('toggle-mod', $event)"
          @delete="emit('delete-mod', $event)"
          @refresh="emit('refresh-mods')"
          @open-folder="emit('open-mods-folder')"
        />
      </TabsContent>
      <TabsContent value="setup" class="flex-1 min-h-0">
        <div class="flex-1 min-h-0 flex flex-col gap-6 overflow-auto pr-1">
          <VersionsCard
            :instance="props.instance"
            :available-versions="props.availableVersions"
            :latest-release="props.latestRelease"
            :installed-versions="props.installedVersions"
            :fabric-loader-versions="props.fabricLoaderVersions"
            :neoforge-loader-versions="props.neoforgeLoaderVersions"
            :working="props.working"
            @update="emit('update-instance', $event)"
            @install="emit('install-version')"
            @refresh="emit('refresh-versions')"
          />
          <InstanceSettingsCard
            :instance="props.instance"
            :instances-count="props.instancesCount"
            :working="props.working"
            @duplicate="emit('duplicate-instance', $event)"
            @remove="emit('remove-instance', $event)"
            @update="emit('update-instance', $event)"
          />
        </div>
      </TabsContent>
    </Tabs>
  </section>
</template>
