<script setup lang="ts">
import Button from "./ui/button/Button.vue";
import Card from "./ui/card/Card.vue";
import CardHeader from "./ui/card/CardHeader.vue";
import CardTitle from "./ui/card/CardTitle.vue";
import CardDescription from "./ui/card/CardDescription.vue";
import CardContent from "./ui/card/CardContent.vue";
import Input from "./ui/input/Input.vue";
import type { DeviceCodeResponse, Profile } from "@/types/auth";

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
      <CardDescription>Sign in once to unlock online play and profiles.</CardDescription>
    </CardHeader>
    <CardContent class="space-y-4">
      <div class="grid gap-3">
        <div class="rounded-2xl border border-border/60 bg-card/70 px-4 py-3 text-sm">
          <div class="text-xs uppercase tracking-widest text-muted-foreground">Status</div>
          <div class="mt-1 font-semibold text-foreground">
            {{ props.profile ? `Signed in as ${props.profile.name}` : "Not signed in" }}
          </div>
        </div>
        <Button :disabled="props.working" @click="emit('start-login')">
          Sign in with Microsoft
        </Button>
        <div
          v-if="props.authFlow === 'device_code' && props.deviceCode"
          class="rounded-2xl border border-border/60 bg-card/70 px-4 py-3 text-sm"
        >
          <div class="text-xs uppercase tracking-widest text-muted-foreground">Step 1</div>
          <div class="mt-1 text-foreground">Open the verification link:</div>
          <div class="mt-2 break-all text-foreground">{{ props.deviceCode.verification_uri }}</div>
          <div class="mt-3 text-xs uppercase tracking-widest text-muted-foreground">Step 2</div>
          <div class="mt-1 text-foreground">Enter this code:</div>
          <div class="mt-2 text-base font-semibold text-foreground">{{ props.deviceCode.user_code }}</div>
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
            class="rounded-2xl border border-border/60 bg-card/70 px-4 py-3 text-sm"
          >
            <div class="text-xs uppercase tracking-widest text-muted-foreground">
              Redirect Received
            </div>
            <div class="mt-1 break-all text-foreground">{{ props.pendingDeeplink }}</div>
          </div>
          <details class="rounded-2xl border border-border/60 bg-card/70 px-4 py-3">
            <summary class="cursor-pointer text-sm font-semibold text-foreground">
              Having trouble with the sign-in link?
            </summary>
            <div class="mt-3 space-y-2">
              <label class="text-xs uppercase tracking-widest text-muted-foreground">
                Paste the callback URL
              </label>
              <Input
                :model-value="props.manualCallbackUrl"
                placeholder="atlas://auth?code=...&state=..."
                @update:modelValue="updateManual"
              />
              <div class="text-xs text-muted-foreground">
                Use this only if the link didn’t open automatically.
              </div>
            </div>
          </details>
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
