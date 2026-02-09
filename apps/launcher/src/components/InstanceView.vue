<script setup lang="ts">
import { computed, ref } from "vue";
import InstanceSettingsCard from "./InstanceSettingsCard.vue";
import ModsCard from "./ModsCard.vue";
import RemoteManageCard from "./RemoteManageCard.vue";
import VersionsCard from "./VersionsCard.vue";
import Button from "./ui/button/Button.vue";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./ui/tabs";
import { Box, ChevronLeftIcon } from "lucide-vue-next";
import type { Profile } from "@/types/auth";
import type { FabricLoaderVersion, ModEntry, VersionSummary } from "@/types/library";
import type { InstanceConfig } from "@/types/settings";
import { formatLoaderKind } from "@/lib/utils";

const props = defineProps<{
  instance: InstanceConfig | null;
  profile: Profile | null;
  working: boolean;
  mods: ModEntry[];
  modsDir: string;
  availableVersions: VersionSummary[];
  latestRelease: string;
  installedVersions: string[];
  fabricLoaderVersions: FabricLoaderVersion[];
  neoforgeLoaderVersions: string[];
  instancesCount: number;
  defaultMemoryMb: number;
  defaultJvmArgs: string;
}>();

const emit = defineEmits<{
  (event: "back"): void;
  (event: "launch"): void;
  (event: "update-files"): void;
  (event: "toggle-mod", payload: { fileName: string; enabled: boolean }): void;
  (event: "delete-mod", fileName: string): void;
  (event: "refresh-mods"): void;
  (event: "open-mods-folder"): void;
  (event: "update-instance", payload: { id: string; patch: Partial<InstanceConfig> }): void;
  (event: "install-version"): void;
  (event: "refresh-versions"): void;
  (event: "duplicate-instance", id: string): void;
  (event: "remove-instance", id: string): void;
  (event: "uninstall-instance"): void;
  (event: "update-channel", value: "dev" | "beta" | "production"): void;
}>();

const detailTab = ref<"content" | "setup">("content");

const activeLoaderLabel = computed(() => {
  const instance = props.instance;
  if (!instance) {
    return "";
  }
  const loader = formatLoaderKind(instance.loader?.kind);
  const version = instance.version?.trim() || "Latest release";
  return `${loader} · ${version}`;
});

const isRemoteInstance = computed(() => props.instance?.source === "atlas");
const hasInstalledFiles = computed(() => props.installedVersions.length > 0);
const needsRemoteInstall = computed(() => isRemoteInstance.value && !hasInstalledFiles.value);
const remoteControlsDisabled = computed(
  () => isRemoteInstance.value && !hasInstalledFiles.value
);
const contentTabLabel = computed(() =>
  isRemoteInstance.value ? "Manage" : "Content"
);
</script>

<template>
  <!-- Fill available height so the tabs area can scroll internally -->
  <section class="flex-1 min-h-0 flex flex-col gap-6 overflow-hidden">
    <div class="glass rounded-2xl px-6 py-4">
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
          <Button v-if="needsRemoteInstall" :disabled="props.working" @click="emit('install-version')">
            Install
          </Button>
          <Button v-else :disabled="props.working || !props.profile" @click="emit('launch')">
            Play
          </Button>
          <Button
            v-if="!needsRemoteInstall"
            :disabled="props.working"
            variant="secondary"
            @click="emit('update-files')"
          >
            Update
          </Button>
        </div>
      </div>
    </div>

    <div
      v-if="!props.profile"
      class="rounded-2xl border border-border/60 bg-card/70 px-4 py-3 text-sm text-muted-foreground"
    >
      Sign in with Microsoft to play. Use the top-right menu to continue setup.
    </div>

    <Tabs v-model="detailTab" class="flex-1 min-h-0 flex flex-col overflow-hidden gap-4">
      <TabsList
        class="flex flex-wrap justify-start gap-2 bg-transparent p-0 shrink-0"
        :class="remoteControlsDisabled ? 'pointer-events-none opacity-50' : ''"
      >
        <TabsTrigger :disabled="remoteControlsDisabled" value="content">
          {{ contentTabLabel }}
        </TabsTrigger>
        <TabsTrigger :disabled="remoteControlsDisabled" value="setup">Setup</TabsTrigger>
        <!-- Logs are available in Settings -->
      </TabsList>
      <TabsContent value="content" class="mt-0 flex-1 min-h-0 overflow-auto pr-1">
        <div
          class="space-y-6"
          :class="remoteControlsDisabled ? 'pointer-events-none select-none opacity-50' : ''"
        >
          <RemoteManageCard
            v-if="isRemoteInstance"
            :instance="props.instance"
            :working="props.working"
            :installed-versions="props.installedVersions"
            @uninstall="emit('uninstall-instance')"
            @update-channel="emit('update-channel', $event)"
          />
          <ModsCard
            v-else
            :instance="props.instance"
            :mods="props.mods"
            :mods-dir="props.modsDir"
            :working="props.working"
            @toggle="emit('toggle-mod', $event)"
            @delete="emit('delete-mod', $event)"
            @refresh="emit('refresh-mods')"
            @open-folder="emit('open-mods-folder')"
          />
        </div>
      </TabsContent>
      <TabsContent value="setup" class="mt-0 flex-1 min-h-0 overflow-auto pr-1">
        <div
          class="flex flex-col gap-6"
          :class="remoteControlsDisabled ? 'pointer-events-none select-none opacity-50' : ''"
        >
          <VersionsCard
            v-if="!isRemoteInstance"
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
            :default-memory-mb="props.defaultMemoryMb"
            :default-jvm-args="props.defaultJvmArgs"
            :working="props.working"
            :managed-by-atlas="isRemoteInstance"
            @duplicate="emit('duplicate-instance', $event)"
            @remove="emit('remove-instance', $event)"
            @update="emit('update-instance', $event)"
          />
        </div>
      </TabsContent>
    </Tabs>
  </section>
</template>
