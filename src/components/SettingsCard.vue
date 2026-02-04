<script setup lang="ts">
import Button from "./ui/button/Button.vue";
import Card from "./ui/card/Card.vue";
import CardHeader from "./ui/card/CardHeader.vue";
import CardTitle from "./ui/card/CardTitle.vue";
import CardDescription from "./ui/card/CardDescription.vue";
import CardContent from "./ui/card/CardContent.vue";
import CardFooter from "./ui/card/CardFooter.vue";
import Input from "./ui/input/Input.vue";

const props = defineProps<{
  settingsClientId: string;
  working: boolean;
}>();

const emit = defineEmits<{
  (event: "update:settingsClientId", value: string): void;
  (event: "save-settings"): void;
}>();

function updateClientId(value: string | number) {
  emit("update:settingsClientId", String(value ?? ""));
}
</script>

<template>
  <Card class="glass">
    <CardHeader>
      <CardTitle>Settings</CardTitle>
      <CardDescription>Optional sign-in overrides.</CardDescription>
    </CardHeader>
    <CardContent class="space-y-3">
      <div class="space-y-2">
        <label class="text-xs uppercase tracking-widest text-muted-foreground">
          Microsoft Client ID (optional)
        </label>
        <Input
          :model-value="props.settingsClientId"
          placeholder="Leave blank to use the bundled client ID"
          @update:modelValue="updateClientId"
        />
      </div>
      <div class="text-xs text-muted-foreground">
        This only affects new sign-ins. Sign out and sign back in to apply.
      </div>
    </CardContent>
    <CardFooter>
      <Button :disabled="props.working" variant="secondary" @click="emit('save-settings')">
        Save settings
      </Button>
    </CardFooter>
  </Card>
</template>
