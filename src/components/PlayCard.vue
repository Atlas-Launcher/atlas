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

const props = defineProps<{
  profile: Profile | null;
  working: boolean;
  progress: number;
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
      <div class="grid gap-3">
        <Button :disabled="props.working" variant="secondary" @click="emit('download')">
          Download game files
        </Button>
        <Button :disabled="props.working || !props.profile" @click="emit('launch')">
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
