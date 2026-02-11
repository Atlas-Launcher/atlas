<script setup lang="ts">
import Card from "./ui/card/Card.vue";
import CardHeader from "./ui/card/CardHeader.vue";
import CardTitle from "./ui/card/CardTitle.vue";
import CardDescription from "./ui/card/CardDescription.vue";
import CardContent from "./ui/card/CardContent.vue";

const props = defineProps<{
  title: string;
  description: string;
  logs: string[];
}>();
</script>

<template>
  <Card class="glass">
    <CardHeader>
      <CardTitle>{{ props.title }}</CardTitle>
      <CardDescription>{{ props.description }}</CardDescription>
    </CardHeader>
    <CardContent>
      <!-- min-w-0 allows this flex child to shrink instead of forcing parent width -->
      <div class="max-h-56 overflow-y-auto pr-1 min-w-0">
        <ul class="space-y-2 text-sm text-muted-foreground">
          <!-- break-all ensures extremely long tokens (no spaces) will wrap to next line -->
          <li v-for="(entry, index) in props.logs" :key="index" class="break-words break-all whitespace-pre-wrap max-w-full">
            {{ entry }}
          </li>
          <li v-if="props.logs.length === 0">Nothing to show yet.</li>
        </ul>
      </div>
    </CardContent>
  </Card>
</template>
