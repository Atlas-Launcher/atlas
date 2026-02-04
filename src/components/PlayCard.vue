<script setup lang="ts">
import Button from "./ui/button/Button.vue";
import Card from "./ui/card/Card.vue";
import CardHeader from "./ui/card/CardHeader.vue";
import CardTitle from "./ui/card/CardTitle.vue";
import CardDescription from "./ui/card/CardDescription.vue";
import CardContent from "./ui/card/CardContent.vue";
import CardFooter from "./ui/card/CardFooter.vue";
import Progress from "./ui/progress/Progress.vue";
import type { Profile } from "@/types/auth";
import type { InstanceConfig } from "@/types/settings";

const props = defineProps<{
  profile: Profile | null;
  working: boolean;
  progress: number;
  instance: InstanceConfig | null;
}>();

const emit = defineEmits<{
  (event: "download"): void;
  (event: "launch"): void;
}>();
</script>

<template>
  <Card class="glass">
    <CardHeader>
      <CardTitle>Play</CardTitle>
      <CardDescription>Atlas manages files and Java for you.</CardDescription>
    </CardHeader>
    <CardContent class="space-y-4">
      <div class="rounded-2xl border border-border/40 bg-secondary/40 px-4 py-3 text-xs">
        <div class="text-xs uppercase tracking-widest text-muted-foreground">Active instance</div>
        <div class="mt-2 font-semibold text-foreground">
          {{ props.instance?.name ?? "No instance selected" }}
        </div>
        <div class="mt-1 text-muted-foreground">
          {{ props.instance?.loader?.kind ?? "vanilla" }}
          ·
          {{ props.instance?.version?.trim() ? props.instance.version : "Latest release" }}
        </div>
      </div>
      <div class="grid gap-3">
        <Button :disabled="props.working || !props.instance" variant="secondary" @click="emit('download')">
          Download game files
        </Button>
        <Button :disabled="props.working || !props.profile || !props.instance" @click="emit('launch')">
          Launch Minecraft
        </Button>
      </div>
      <div class="text-xs text-muted-foreground">
        Files are stored in your app data folder and updated automatically.
      </div>
    </CardContent>
    <CardFooter>
      <div class="w-full space-y-2">
        <div class="flex items-center justify-between text-xs text-muted-foreground">
          <span>Launch progress</span>
          <span>{{ props.progress }}%</span>
        </div>
        <Progress :model-value="props.progress" />
      </div>
    </CardFooter>
  </Card>
</template>
