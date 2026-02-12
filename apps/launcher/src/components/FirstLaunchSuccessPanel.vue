<script setup lang="ts">
import { CheckCircle2, LifeBuoy, Play } from "lucide-vue-next";
import Button from "./ui/button/Button.vue";
import Card from "./ui/card/Card.vue";
import CardContent from "./ui/card/CardContent.vue";

const props = defineProps<{
  open: boolean;
  packName?: string | null;
}>();

const emit = defineEmits<{
  (event: "retry-launch"): void;
  (event: "open-assist"): void;
  (event: "dismiss"): void;
}>();
</script>

<template>
  <Card
    v-if="props.open"
    class="mx-4 rounded-2xl border border-emerald-500/30 bg-emerald-500/10"
    role="status"
    aria-live="polite"
  >
    <CardContent class="py-4">
      <div class="flex flex-wrap items-start justify-between gap-3">
        <div class="min-w-0">
          <p class="flex items-center gap-2 text-sm font-semibold text-emerald-700 dark:text-emerald-300">
            <CheckCircle2 class="h-4 w-4" />
            First launch complete
          </p>
          <p class="mt-1 text-xs text-muted-foreground">
            {{ props.packName ? `${props.packName} launched successfully.` : "Your profile launched successfully." }}
          </p>
          <p class="mt-1 text-xs text-muted-foreground">
            You can launch again anytime, or open Launch Assist if you need help.
          </p>
        </div>
        <div class="flex flex-wrap items-center gap-2">
          <Button size="sm" @click="emit('retry-launch')">
            <Play class="mr-1 h-3.5 w-3.5" />
            Play again
          </Button>
          <Button size="sm" variant="outline" @click="emit('open-assist')">
            <LifeBuoy class="mr-1 h-3.5 w-3.5" />
            Launch Assist
          </Button>
          <Button size="sm" variant="ghost" @click="emit('dismiss')">Close</Button>
        </div>
      </div>
    </CardContent>
  </Card>
</template>
