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
import type { InstanceConfig, ModLoaderKind } from "@/types/settings";
import type { FabricLoaderVersion, VersionSummary } from "@/types/library";

const props = defineProps<{
  instance: InstanceConfig | null;
  availableVersions: VersionSummary[];
  latestRelease: string;
  installedVersions: string[];
  fabricLoaderVersions: FabricLoaderVersion[];
  working: boolean;
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

const isInstalled = computed(() => {
  if (!props.instance?.version) {
    return false;
  }
  return props.installedVersions.includes(props.instance.version);
});

function updateLoaderKind(kind: ModLoaderKind) {
  if (!props.instance) {
    return;
  }
  const loader = {
    ...props.instance.loader,
    kind,
    loaderVersion: kind === "neoforge" ? props.instance.loader.loaderVersion ?? null : null
  };

  const patch: Partial<InstanceConfig> = { loader };
  if (kind === "fabric" && !props.instance.version && props.latestRelease) {
    patch.version = props.latestRelease;
  }
  emit("update", { id: props.instance.id, patch });
}

function updateVersion(version: string) {
  if (!props.instance) {
    return;
  }
  emit("update", { id: props.instance.id, patch: { version } });
}

function updateFabricLoaderVersion(version: string) {
  if (!props.instance) {
    return;
  }
  const loader = {
    ...props.instance.loader,
    loaderVersion: version.trim().length > 0 ? version : null
  };
  emit("update", { id: props.instance.id, patch: { loader } });
}

function updateNeoForgeVersion(value: string | number) {
  if (!props.instance) {
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
  <Card class="glass">
    <CardHeader>
      <CardTitle>Versions</CardTitle>
      <CardDescription>Pick the Minecraft version and loader per instance.</CardDescription>
    </CardHeader>
    <CardContent class="space-y-4">
      <div v-if="!props.instance" class="text-sm text-muted-foreground">
        Select an instance to manage versions.
      </div>

      <div v-else class="space-y-4">
        <div class="grid gap-4 md:grid-cols-2">
          <div class="space-y-2">
            <label class="text-xs uppercase tracking-widest text-muted-foreground">
              Mod loader
            </label>
            <div class="grid grid-cols-3 gap-2 text-xs font-semibold">
              <button
                class="rounded-xl border px-3 py-2 transition"
                :class="
                  props.instance.loader.kind === 'vanilla'
                    ? 'border-primary/60 bg-primary/10 text-foreground'
                    : 'border-border/40 bg-secondary/40 text-muted-foreground hover:text-foreground'
                "
                @click="updateLoaderKind('vanilla')"
              >
                Vanilla
              </button>
              <button
                class="rounded-xl border px-3 py-2 transition"
                :class="
                  props.instance.loader.kind === 'fabric'
                    ? 'border-primary/60 bg-primary/10 text-foreground'
                    : 'border-border/40 bg-secondary/40 text-muted-foreground hover:text-foreground'
                "
                @click="updateLoaderKind('fabric')"
              >
                Fabric
              </button>
              <button
                class="rounded-xl border px-3 py-2 transition"
                :class="
                  props.instance.loader.kind === 'neoforge'
                    ? 'border-primary/60 bg-primary/10 text-foreground'
                    : 'border-border/40 bg-secondary/40 text-muted-foreground hover:text-foreground'
                "
                @click="updateLoaderKind('neoforge')"
              >
                NeoForge
              </button>
            </div>
          </div>
          <div class="space-y-2">
            <label class="text-xs uppercase tracking-widest text-muted-foreground">
              Minecraft version
            </label>
            <select
              class="h-9 w-full rounded-md border border-input bg-transparent px-3 text-sm shadow-sm transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
              :value="props.instance.version ?? ''"
              @change="updateVersion(($event.target as HTMLSelectElement).value)"
            >
              <option value="">
                Latest release ({{ props.latestRelease || "auto" }})
              </option>
              <optgroup label="Releases">
                <option v-for="version in releaseVersions" :key="version.id" :value="version.id">
                  {{ version.id }}
                </option>
              </optgroup>
              <optgroup v-if="snapshotVersions.length" label="Snapshots">
                <option
                  v-for="version in snapshotVersions"
                  :key="version.id"
                  :value="version.id"
                >
                  {{ version.id }}
                </option>
              </optgroup>
            </select>
          </div>
        </div>

        <div v-if="props.instance.loader.kind === 'fabric'" class="space-y-2">
          <label class="text-xs uppercase tracking-widest text-muted-foreground">
            Fabric loader version
          </label>
          <select
            class="h-9 w-full rounded-md border border-input bg-transparent px-3 text-sm shadow-sm transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
            :value="props.instance.loader.loaderVersion ?? ''"
            @change="updateFabricLoaderVersion(($event.target as HTMLSelectElement).value)"
          >
            <option value="">Latest stable</option>
            <option
              v-for="loader in props.fabricLoaderVersions"
              :key="loader.version"
              :value="loader.version"
            >
              {{ loader.version }} {{ loader.stable ? "" : "(beta)" }}
            </option>
          </select>
        </div>

        <div v-if="props.instance.loader.kind === 'neoforge'" class="space-y-2">
          <label class="text-xs uppercase tracking-widest text-muted-foreground">
            NeoForge profile id
          </label>
          <Input
            :model-value="props.instance.loader.loaderVersion ?? ''"
            placeholder="Example: 20.4.205 (expects neoforge-20.4.205 profile)"
            @update:modelValue="updateNeoForgeVersion"
          />
          <p class="text-xs text-muted-foreground">
            NeoForge must be installed into this instance before launch. Atlas looks for a matching
            profile in the versions folder.
          </p>
        </div>

        <div class="rounded-2xl border border-border/40 bg-secondary/40 px-4 py-3 text-xs">
          <div class="flex flex-wrap items-center justify-between gap-2">
            <div>
              <div class="uppercase tracking-widest text-muted-foreground">Installed</div>
              <div class="mt-1 font-semibold text-foreground">
                {{ isInstalled ? "Selected version installed" : "Not installed yet" }}
              </div>
            </div>
            <Button :disabled="props.working" size="sm" variant="secondary" @click="emit('refresh')">
              Refresh
            </Button>
          </div>
          <div class="mt-2 flex flex-wrap gap-2 text-muted-foreground">
            <span v-for="version in props.installedVersions" :key="version" class="rounded-full border px-2 py-1">
              {{ version }}
            </span>
            <span v-if="props.installedVersions.length === 0">No versions installed.</span>
          </div>
        </div>
      </div>
    </CardContent>
    <CardFooter class="flex items-center justify-between">
      <div class="text-xs text-muted-foreground">
        Install the selected version into the instance folder.
      </div>
      <Button :disabled="props.working || !props.instance" variant="secondary" @click="emit('install')">
        Install / Update
      </Button>
    </CardFooter>
  </Card>
</template>
