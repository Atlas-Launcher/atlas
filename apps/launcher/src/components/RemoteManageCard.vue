<script setup lang="ts">
import { computed } from "vue";
import Button from "./ui/button/Button.vue";
import Card from "./ui/card/Card.vue";
import CardContent from "./ui/card/CardContent.vue";
import CardDescription from "./ui/card/CardDescription.vue";
import CardFooter from "./ui/card/CardFooter.vue";
import CardHeader from "./ui/card/CardHeader.vue";
import CardTitle from "./ui/card/CardTitle.vue";
import type { InstanceConfig } from "@/types/settings";

const props = defineProps<{
  instance: InstanceConfig | null;
  working: boolean;
  installedVersions: string[];
}>();

const emit = defineEmits<{
  (event: "install"): void;
  (event: "refresh"): void;
  (event: "uninstall"): void;
}>();

const hasInstalledFiles = computed(() => props.installedVersions.length > 0);
</script>

<template>
  <Card class="glass">
    <CardHeader>
      <CardTitle>Manage remote pack</CardTitle>
      <CardDescription>
        Install, update, or remove local files for this Atlas Hub pack.
      </CardDescription>
    </CardHeader>
    <CardContent class="space-y-4">
      <div v-if="!props.instance" class="text-sm text-muted-foreground">
        Select a profile to manage.
      </div>

      <div v-else class="space-y-4">
        <div class="rounded-2xl border border-border/60 bg-card/70 px-4 py-3 text-sm">
          <div class="text-xs uppercase tracking-widest text-muted-foreground">Remote source</div>
          <div class="mt-1 font-semibold text-foreground">
            {{ props.instance.atlasPack?.packSlug ?? props.instance.name }}
          </div>
          <div class="mt-1 text-xs text-muted-foreground">
            Channel: {{ props.instance.atlasPack?.channel?.toUpperCase() ?? "PRODUCTION" }}
            <template v-if="props.instance.atlasPack?.buildVersion">
              · Build {{ props.instance.atlasPack.buildVersion }}
            </template>
          </div>
        </div>

        <div class="rounded-2xl border border-border/60 bg-card/70 px-4 py-3 text-sm">
          <div class="text-xs uppercase tracking-widest text-muted-foreground">Install state</div>
          <div class="mt-1 font-semibold text-foreground">
            {{ hasInstalledFiles ? "Installed locally" : "Not installed locally" }}
          </div>
          <div class="mt-1 text-xs text-muted-foreground">
            Installed game versions: {{ props.installedVersions.length }}
          </div>
        </div>

        <div class="flex flex-wrap gap-2">
          <Button :disabled="props.working" variant="secondary" @click="emit('install')">
            {{ hasInstalledFiles ? "Update" : "Install" }}
          </Button>
          <Button :disabled="props.working" variant="secondary" @click="emit('refresh')">
            Refresh state
          </Button>
          <Button :disabled="props.working" variant="destructive" @click="emit('uninstall')">
            Uninstall local files
          </Button>
        </div>
      </div>
    </CardContent>
    <CardFooter>
      <div class="text-xs text-muted-foreground">
        Version and loader are managed by Atlas Hub for remote packs.
      </div>
    </CardFooter>
  </Card>
</template>
