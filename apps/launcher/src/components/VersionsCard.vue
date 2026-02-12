<script setup lang="ts">
import { computed } from "vue";
import Button from "./ui/button/Button.vue";
import Card from "./ui/card/Card.vue";
import CardHeader from "./ui/card/CardHeader.vue";
import CardTitle from "./ui/card/CardTitle.vue";
import CardDescription from "./ui/card/CardDescription.vue";
import CardContent from "./ui/card/CardContent.vue";
import CardFooter from "./ui/card/CardFooter.vue";
import Input from "./ui/input/Input.vue";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue
} from "./ui/select";
import type { InstanceConfig, ModLoaderKind } from "@/types/settings";
import type { FabricLoaderVersion, NeoForgeLoaderVersion, VersionSummary } from "@/types/library";

const props = defineProps<{
  instance: InstanceConfig | null;
  availableVersions: VersionSummary[];
  latestRelease: string;
  installedVersions: string[];
  fabricLoaderVersions: FabricLoaderVersion[];
  neoforgeLoaderVersions: NeoForgeLoaderVersion[];
  working: boolean;
  setupLocked?: boolean;
}>();

const emit = defineEmits<{
  (event: "update", payload: { id: string; patch: Partial<InstanceConfig> }): void;
  (event: "install"): void;
  (event: "refresh"): void;
}>();

const releaseVersions = computed(() =>
  props.availableVersions.filter((version) => version.kind === "release")
);
const snapshotVersions = computed(() =>
  props.availableVersions.filter((version) => version.kind !== "release")
);
const defaultNeoForgeVersion = computed(() => props.neoforgeLoaderVersions[0] ?? null);

const isInstalled = computed(() => {
  if (!props.instance?.version) {
    return false;
  }
  return props.installedVersions.includes(props.instance.version);
});

function updateLoaderKind(kind: ModLoaderKind) {
  if (!props.instance || props.setupLocked) {
    return;
  }
  const loader = {
    ...props.instance.loader,
    kind,
    loaderVersion:
      kind === "neoforge"
        ? props.instance.loader.loaderVersion ?? defaultNeoForgeVersion.value ?? null
        : null
  };

  const patch: Partial<InstanceConfig> = { loader };
  if (kind === "fabric" && !props.instance.version && props.latestRelease) {
    patch.version = props.latestRelease;
  }
  emit("update", { id: props.instance.id, patch });
}

function updateVersion(version: any) {
  if (!props.instance || props.setupLocked) {
    return;
  }
  const value = version === "latest" || version === null ? null : String(version);
  emit("update", { id: props.instance.id, patch: { version: value } });
}

function updateFabricLoaderVersion(version: any) {
  if (!props.instance || props.setupLocked) {
    return;
  }
  const loader = {
    ...props.instance.loader,
    loaderVersion: version === "latest" || version === null ? null : String(version).trim().length > 0 ? String(version) : null
  };
  emit("update", { id: props.instance.id, patch: { loader } });
}

function updateNeoForgeVersion(value: string | number) {
  if (!props.instance || props.setupLocked) {
    return;
  }
  const loader = {
    ...props.instance.loader,
    loaderVersion: String(value ?? "").trim() || null
  };
  emit("update", { id: props.instance.id, patch: { loader } });
}
</script>

<template>
  <Card class="glass h-full min-h-0 rounded-2xl flex flex-col">
    <CardHeader class="pt-7">
      <CardTitle>Game setup</CardTitle>
      <CardDescription>
        {{
          props.setupLocked
            ? "This setup is managed in Atlas Hub and cannot be edited locally."
            : "Choose your Minecraft version and mod loader."
        }}
      </CardDescription>
    </CardHeader>
    <CardContent class="flex-1 min-h-0 overflow-y-auto space-y-4 pr-3 pb-5 pt-1 [scrollbar-gutter:stable]">
      <div v-if="!props.instance" class="text-sm text-muted-foreground">
        Select a profile to manage game setup.
      </div>

      <div v-else class="space-y-4">
        <div class="grid gap-4 md:grid-cols-2">
          <div class="space-y-2">
            <label class="text-xs uppercase tracking-widest text-muted-foreground">
              Mod loader
            </label>
            <div class="grid grid-cols-3 gap-2 text-xs font-semibold">
              <button
                :disabled="props.setupLocked"
                class="rounded-xl border px-3 py-2 text-left transition"
                :class="
                  props.instance.loader.kind === 'vanilla'
                    ? 'border-foreground/70 bg-foreground/5 text-foreground'
                    : 'border-border/60 bg-card/70 text-muted-foreground hover:text-foreground'
                "
                @click="updateLoaderKind('vanilla')"
              >
                <div>Vanilla</div>
                <div class="text-[10px] uppercase tracking-widest text-muted-foreground">
                  Official
                </div>
              </button>
              <button
                :disabled="props.setupLocked"
                class="rounded-xl border px-3 py-2 text-left transition"
                :class="
                  props.instance.loader.kind === 'fabric'
                    ? 'border-foreground/70 bg-foreground/5 text-foreground'
                    : 'border-border/60 bg-card/70 text-muted-foreground hover:text-foreground'
                "
                @click="updateLoaderKind('fabric')"
              >
                <div>Fabric</div>
                <div class="text-[10px] uppercase tracking-widest text-muted-foreground">
                  Lightweight mods
                </div>
              </button>
              <button
                :disabled="props.setupLocked"
                class="rounded-xl border px-3 py-2 text-left transition"
                :class="
                  props.instance.loader.kind === 'neoforge'
                    ? 'border-foreground/70 bg-foreground/5 text-foreground'
                    : 'border-border/60 bg-card/70 text-muted-foreground hover:text-foreground'
                "
                @click="updateLoaderKind('neoforge')"
              >
                <div>NeoForge</div>
                <div class="text-[10px] uppercase tracking-widest text-muted-foreground">
                  Forge ecosystem
                </div>
              </button>
            </div>
          </div>
          <div class="space-y-2">
            <label class="text-xs uppercase tracking-widest text-muted-foreground">
              Minecraft version
            </label>
            <Select
              :disabled="props.setupLocked"
              :model-value="props.instance.version ?? 'latest'"
              @update:modelValue="updateVersion"
            >
              <SelectTrigger>
                <SelectValue placeholder="Latest release" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="latest">
                  Latest release ({{ props.latestRelease || "auto" }})
                </SelectItem>
                <SelectGroup>
                  <SelectLabel>Releases</SelectLabel>
                  <SelectItem
                    v-for="version in releaseVersions"
                    :key="version.id"
                    :value="version.id"
                  >
                    {{ version.id }}
                  </SelectItem>
                </SelectGroup>
                <SelectGroup v-if="snapshotVersions.length">
                  <SelectLabel>Snapshots</SelectLabel>
                  <SelectItem
                    v-for="version in snapshotVersions"
                    :key="version.id"
                    :value="version.id"
                  >
                    {{ version.id }}
                  </SelectItem>
                </SelectGroup>
              </SelectContent>
            </Select>
          </div>
        </div>

        <div v-if="props.instance.loader.kind === 'fabric'" class="space-y-2">
          <label class="text-xs uppercase tracking-widest text-muted-foreground">
            Fabric loader version (optional)
          </label>
          <Select
            :disabled="props.setupLocked"
            :model-value="props.instance.loader.loaderVersion ?? 'latest'"
            @update:modelValue="updateFabricLoaderVersion"
          >
            <SelectTrigger>
              <SelectValue placeholder="Latest stable" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="latest">Latest stable</SelectItem>
              <SelectItem
                v-for="loader in props.fabricLoaderVersions"
                :key="loader.version"
                :value="loader.version"
              >
                {{ loader.version }} {{ loader.stable ? "" : "(beta)" }}
              </SelectItem>
            </SelectContent>
          </Select>
        </div>

        <div v-if="props.instance.loader.kind === 'neoforge'" class="space-y-2">
          <label class="text-xs uppercase tracking-widest text-muted-foreground">
            NeoForge loader version
          </label>
          <Input
            :disabled="props.setupLocked"
            :model-value="props.instance.loader.loaderVersion ?? ''"
            list="neoforge-versions"
            placeholder="Pick a version from the list"
            @update:modelValue="updateNeoForgeVersion"
          />
          <datalist id="neoforge-versions">
            <option v-for="version in props.neoforgeLoaderVersions" :key="version" :value="version">
              {{ version }}
            </option>
          </datalist>
          <p class="text-xs text-muted-foreground">
            Atlas applies the matching setup automatically.
          </p>
        </div>

        <div class="rounded-2xl border border-border/60 bg-card/70 px-4 py-3 text-xs">
          <div class="flex flex-wrap items-center justify-between gap-2">
            <div>
              <div class="uppercase tracking-widest text-muted-foreground">Installed versions</div>
              <div class="mt-1 font-semibold text-foreground">
                {{ isInstalled ? "Selected setup is installed" : "Setup not installed yet" }}
              </div>
            </div>
            <Button :disabled="props.working" size="sm" variant="secondary" @click="emit('refresh')">
              Refresh list
            </Button>
          </div>
          <div class="mt-2 flex flex-wrap gap-2 text-muted-foreground">
            <span v-for="version in props.installedVersions" :key="version" class="rounded-full border px-2 py-1">
              {{ version }}
            </span>
            <span v-if="props.installedVersions.length === 0">No versions installed yet.</span>
          </div>
        </div>
      </div>
    </CardContent>
    <CardFooter class="flex items-center justify-between pt-0">
      <div class="text-xs text-muted-foreground">
        {{
          props.setupLocked
            ? "Remote packs are installed and updated from Manage pack."
            : "Install the selected setup."
        }}
      </div>
      <Button
        :disabled="props.setupLocked || props.working || !props.instance"
        variant="secondary"
        @click="emit('install')"
      >
        Install setup
      </Button>
    </CardFooter>
  </Card>
</template>
