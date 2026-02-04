<script setup lang="ts">
import Button from "./ui/button/Button.vue";
import Card from "./ui/card/Card.vue";
import CardHeader from "./ui/card/CardHeader.vue";
import CardTitle from "./ui/card/CardTitle.vue";
import CardDescription from "./ui/card/CardDescription.vue";
import CardContent from "./ui/card/CardContent.vue";
import Input from "./ui/input/Input.vue";

interface Profile {
  id: string;
  name: string;
}

interface DeviceCodeResponse {
  device_code: string;
  user_code: string;
  verification_uri: string;
}

const props = defineProps<{
  profile: Profile | null;
  working: boolean;
  authFlow: string;
  deviceCode: DeviceCodeResponse | null;
  pendingDeeplink: string | null;
  manualCallbackUrl: string;
}>();

const emit = defineEmits<{
  (event: "start-login"): void;
  (event: "complete-device-login"): void;
  (event: "finish-deeplink-login"): void;
  (event: "sign-out"): void;
  (event: "update:manualCallbackUrl", value: string): void;
}>();

function updateManual(value: string | number) {
  emit("update:manualCallbackUrl", String(value ?? ""));
}
</script>

<template>
  <Card class="glass">
    <CardHeader>
      <CardTitle>Account</CardTitle>
      <CardDescription>Connect your Microsoft account to play.</CardDescription>
    </CardHeader>
    <CardContent class="space-y-4">
      <div class="grid gap-3">
        <div class="rounded-lg border border-border bg-secondary/40 px-4 py-3 text-sm">
          <div class="text-xs uppercase tracking-widest text-muted-foreground">Status</div>
          <div class="mt-1 font-semibold text-foreground">
            {{ props.profile ? `Signed in as ${props.profile.name}` : "Not signed in" }}
          </div>
        </div>
        <Button :disabled="props.working" @click="emit('start-login')">Sign in</Button>
        <div
          v-if="props.authFlow === 'device_code' && props.deviceCode"
          class="rounded-lg border border-border bg-muted/40 px-4 py-3 text-sm"
        >
          <div class="text-xs uppercase tracking-widest text-muted-foreground">Verification URL</div>
          <div class="mt-1 break-all text-foreground">{{ props.deviceCode.verification_uri }}</div>
          <div class="mt-3 text-xs uppercase tracking-widest text-muted-foreground">User Code</div>
          <div class="mt-1 text-base font-semibold text-foreground">{{ props.deviceCode.user_code }}</div>
        </div>
        <Button
          v-if="props.authFlow === 'device_code' && props.deviceCode"
          :disabled="props.working"
          variant="secondary"
          @click="emit('complete-device-login')"
        >
          Complete sign-in
        </Button>
        <div v-if="props.authFlow !== 'device_code' && !props.profile" class="space-y-2">
          <div
            v-if="props.pendingDeeplink"
            class="rounded-lg border border-border bg-secondary/40 px-4 py-3 text-sm"
          >
            <div class="text-xs uppercase tracking-widest text-muted-foreground">
              Redirect Received
            </div>
            <div class="mt-1 break-all text-foreground">{{ props.pendingDeeplink }}</div>
          </div>
          <div class="space-y-2">
            <label class="text-xs uppercase tracking-widest text-muted-foreground">
              Auth Callback URL (optional)
            </label>
            <Input
              :model-value="props.manualCallbackUrl"
              placeholder="atlas://auth?code=...&state=..."
              @update:modelValue="updateManual"
            />
            <div class="text-xs text-muted-foreground">
              Use this if the deep link didn't open automatically.
            </div>
          </div>
          <Button :disabled="props.working" variant="secondary" @click="emit('finish-deeplink-login')">
            Finish sign-in
          </Button>
        </div>
        <Button v-if="props.profile" variant="ghost" :disabled="props.working" @click="emit('sign-out')">
          Sign out
        </Button>
      </div>
    </CardContent>
  </Card>
</template>
