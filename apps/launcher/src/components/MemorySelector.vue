<script setup lang="ts">
import { computed, ref, watch } from "vue";
import Button from "./ui/button/Button.vue";
import Input from "./ui/input/Input.vue";

const props = withDefaults(defineProps<{
  title: string;
  modelValue: number;
  maxMb?: number | null;
  recommendedMb?: number | null;
  systemMemoryMb?: number | null;
  working?: boolean;
  showRecommended?: boolean;
  showLimitsCopy?: boolean;
}>(), {
  maxMb: null,
  recommendedMb: null,
  systemMemoryMb: null,
  working: false,
  showRecommended: true,
  showLimitsCopy: false
});

const emit = defineEmits<{
  (event: "update:modelValue", value: number): void;
}>();

const MEMORY_MIN_MB = 1024;
const MEMORY_STEP_MB = 512;
const MEMORY_RECOMMENDED_FALLBACK_MB = 12 * 1024;
const MEMORY_MARKS_GB = [2, 4, 6, 8, 12, 16, 24, 32] as const;
const MEMORY_SNAP_THRESHOLD_MB = 256;
const SYSTEM_MEMORY_RESERVED_MB = 2 * 1024;
const HIGH_RAM_THRESHOLD_MB = 34 * 1024;
const HIGH_RAM_CAP_MB = 32 * 1024;

function formatGbFloorFromMb(valueMb: number) {
  return `${Math.floor(valueMb / 1024)} GB`;
}

function resolveCapMb() {
  const cap = props.maxMb;
  if (typeof cap !== "number" || !Number.isFinite(cap) || cap <= 0) {
    return null;
  }
  return Math.max(MEMORY_MIN_MB, Math.floor(cap / MEMORY_STEP_MB) * MEMORY_STEP_MB);
}

function normalizeMemoryValue(value: number | string) {
  const parsed = Number(value);
  const base = Number.isFinite(parsed)
    ? Math.max(MEMORY_MIN_MB, Math.round(parsed))
    : Math.max(MEMORY_MIN_MB, Math.round(props.modelValue || MEMORY_MIN_MB));
  const roundedUp = Math.max(MEMORY_MIN_MB, Math.ceil(base / MEMORY_STEP_MB) * MEMORY_STEP_MB);
  const cap = resolveCapMb();
  if (cap == null) {
    return roundedUp;
  }
  if (roundedUp <= cap) {
    return roundedUp;
  }
  return cap;
}

const sliderMaxMb = computed(() => {
  const cap = resolveCapMb();
  if (cap != null) {
    return cap;
  }
  const fallback = Math.max(
    Math.max(MEMORY_MIN_MB, Math.round(props.modelValue || MEMORY_MIN_MB)),
    MEMORY_RECOMMENDED_FALLBACK_MB
  );
  return Math.max(MEMORY_MIN_MB, Math.ceil(fallback / MEMORY_STEP_MB) * MEMORY_STEP_MB);
});

const sliderMemoryValue = computed(() => {
  const normalized = normalizeMemoryValue(props.modelValue);
  return Math.min(sliderMaxMb.value, Math.max(MEMORY_MIN_MB, normalized));
});

const isSliderDragging = ref(false);
const sliderDragValueMb = ref<number | null>(null);
const displayedSliderValue = computed(() =>
  isSliderDragging.value && sliderDragValueMb.value != null
    ? sliderDragValueMb.value
    : sliderMemoryValue.value
);

const memoryInputValue = ref(String(sliderMemoryValue.value));
watch(
  () => sliderMemoryValue.value,
  (value) => {
    if (isSliderDragging.value) {
      return;
    }
    memoryInputValue.value = String(value);
  }
);

const sliderProgress = computed(() => {
  const span = sliderMaxMb.value - MEMORY_MIN_MB;
  if (span <= 0) {
    return 100;
  }
  return ((displayedSliderValue.value - MEMORY_MIN_MB) / span) * 100;
});

const sliderStyle = computed(() => ({
  "--memory-slider-fill": `${Math.max(0, Math.min(100, sliderProgress.value))}%`,
  "--memory-track-inset": "7px"
}));

type SliderMark = {
  key: string;
  label: string;
  mb: number;
  percent: number;
  edge: "left" | "right" | "center";
};

type SliderStepTick = {
  mb: number;
  percent: number;
  edge: "left" | "right" | "center";
};

const sliderStepTicks = computed<SliderStepTick[]>(() => {
  const span = sliderMaxMb.value - MEMORY_MIN_MB;
  if (span <= 0) {
    return [];
  }
  const ticks: SliderStepTick[] = [];
  for (let mb = MEMORY_MIN_MB; mb <= sliderMaxMb.value; mb += MEMORY_STEP_MB) {
    const percent = ((mb - MEMORY_MIN_MB) / span) * 100;
    ticks.push({
      mb,
      percent,
      edge: percent <= 2 ? "left" : percent >= 98 ? "right" : "center"
    });
  }
  return ticks;
});

const sliderMarks = computed<SliderMark[]>(() => {
  const span = sliderMaxMb.value - MEMORY_MIN_MB;
  if (span <= 0) {
    return [];
  }
  const byMb = new Map<number, SliderMark>();
  const upsertMark = (mb: number, label: string, key: string) => {
    if (mb < MEMORY_MIN_MB || mb > sliderMaxMb.value) {
      return;
    }
    const percent = ((mb - MEMORY_MIN_MB) / span) * 100;
    byMb.set(mb, {
      key,
      label,
      mb,
      percent,
      edge: percent <= 2 ? "left" : percent >= 98 ? "right" : "center"
    });
  };

  for (const gb of MEMORY_MARKS_GB) {
    upsertMark(gb * 1024, `${gb} GB`, `notable-${gb}`);
  }

  byMb.set(MEMORY_MIN_MB, {
    key: "range-min",
    label: formatGbFloorFromMb(MEMORY_MIN_MB),
    mb: MEMORY_MIN_MB,
    percent: 0,
    edge: "center"
  });
  byMb.set(sliderMaxMb.value, {
    key: "range-max",
    label: formatGbFloorFromMb(sliderMaxMb.value),
    mb: sliderMaxMb.value,
    percent: 100,
    edge: "center"
  });

  return [...byMb.values()].sort((a, b) => a.mb - b.mb);
});

const normalizedRecommendedMemoryMb = computed(() => {
  const value = props.recommendedMb;
  if (typeof value !== "number" || !Number.isFinite(value) || value <= 0) {
    return null;
  }
  return normalizeMemoryValue(value);
});

const canApplyRecommendedMemory = computed(
  () =>
    props.showRecommended &&
    normalizedRecommendedMemoryMb.value != null &&
    normalizedRecommendedMemoryMb.value !== sliderMemoryValue.value
);

function snapSliderValueToNotableMemory(value: number) {
  const cap = sliderMaxMb.value;
  const candidates = MEMORY_MARKS_GB
    .map((gb) => gb * 1024)
    .filter((mb) => mb >= MEMORY_MIN_MB && mb <= cap);
  if (candidates.length === 0) {
    return value;
  }

  let closest = value;
  let closestDistance = Number.POSITIVE_INFINITY;
  for (const candidate of candidates) {
    const distance = Math.abs(candidate - value);
    if (distance < closestDistance) {
      closest = candidate;
      closestDistance = distance;
    }
  }

  if (closestDistance <= MEMORY_SNAP_THRESHOLD_MB) {
    return closest;
  }
  return value;
}

function commitMemoryValue(value: number | string) {
  const normalized = normalizeMemoryValue(value);
  emit("update:modelValue", normalized);
  memoryInputValue.value = String(normalized);
}

function startSliderDrag() {
  isSliderDragging.value = true;
  sliderDragValueMb.value = displayedSliderValue.value;
}

function finishSliderDrag() {
  if (!isSliderDragging.value) {
    return;
  }
  const base = sliderDragValueMb.value ?? sliderMemoryValue.value;
  const snapped = snapSliderValueToNotableMemory(base);
  commitMemoryValue(snapped);
  sliderDragValueMb.value = null;
  isSliderDragging.value = false;
}

function updateFromSlider(event: Event) {
  const target = event.target as HTMLInputElement | null;
  const raw = Number(target?.value ?? props.modelValue);
  const normalized = normalizeMemoryValue(raw);
  isSliderDragging.value = true;
  sliderDragValueMb.value = normalized;
  memoryInputValue.value = String(normalized);
  emit("update:modelValue", normalized);
}

function updateFromInput(value: string | number) {
  isSliderDragging.value = false;
  sliderDragValueMb.value = null;
  memoryInputValue.value = String(value ?? "");
  commitMemoryValue(memoryInputValue.value);
}

function applyRecommendedMemory() {
  if (normalizedRecommendedMemoryMb.value == null) {
    return;
  }
  commitMemoryValue(normalizedRecommendedMemoryMb.value);
}

function formatGbFloor(value: number | null | undefined) {
  if (typeof value !== "number" || !Number.isFinite(value) || value <= 0) {
    return "0 GB";
  }
  return `${Math.floor(value / 1024)} GB`;
}

const memoryLimitCopy = computed(() => {
  if (!props.showLimitsCopy) {
    return null;
  }

  const systemMb = props.systemMemoryMb;
  if (typeof systemMb !== "number" || !Number.isFinite(systemMb) || systemMb <= 0) {
    return "System memory is unavailable, so only launcher safety limits are shown.";
  }

  const totalGb = Math.floor(systemMb / 1024);
  const reserveApplies = systemMb > SYSTEM_MEMORY_RESERVED_MB;
  const highRamCapApplies = systemMb >= HIGH_RAM_THRESHOLD_MB && sliderMaxMb.value <= HIGH_RAM_CAP_MB;

  const limits: string[] = [];
  if (reserveApplies) {
    limits.push("2 GB is reserved for the OS and background processes");
  }
  if (highRamCapApplies) {
    limits.push("max is capped at 32 GB on high-memory systems");
  }

  if (limits.length === 0) {
    return `System memory: ${totalGb} GB.`;
  }
  return `System memory: ${totalGb} GB. ${limits.join("; ")}.`;
});
</script>

<template>
  <div class="space-y-2">
    <div class="flex flex-wrap items-center justify-between gap-2">
      <label class="text-xs uppercase tracking-widest text-muted-foreground">
        {{ props.title }}
      </label>
      <Button
        v-if="props.showRecommended"
        size="sm"
        variant="outline"
        :disabled="props.working || !canApplyRecommendedMemory"
        @click="applyRecommendedMemory"
      >
        Use recommended
        <span v-if="normalizedRecommendedMemoryMb" class="ml-1 text-xs text-muted-foreground">
          ({{ formatGbFloor(normalizedRecommendedMemoryMb) }})
        </span>
      </Button>
    </div>
    <div class="rounded-xl border border-border/70 bg-card/60 px-3 pb-3 pt-2.5">
      <div class="flex items-start gap-3">
        <div class="memory-slider-stack" :style="sliderStyle">
          <input
            class="memory-slider"
            type="range"
            :min="MEMORY_MIN_MB"
            :max="sliderMaxMb"
            :step="MEMORY_STEP_MB"
            :value="displayedSliderValue"
            @input="updateFromSlider"
            @pointerdown="startSliderDrag"
            @pointerup="finishSliderDrag"
            @pointercancel="finishSliderDrag"
            @change="finishSliderDrag"
          />
          <div class="memory-slider-track-geometry">
            <div
              v-if="sliderStepTicks.length > 0 || sliderMarks.length > 0"
              class="memory-slider-marking-layer mt-2"
            >
              <span
                v-for="tick in sliderStepTicks"
                :key="tick.mb"
                class="memory-slider-step-tick"
                :class="{
                  'memory-slider-step-tick-left': tick.edge === 'left',
                  'memory-slider-step-tick-right': tick.edge === 'right'
                }"
                :style="{ left: `${tick.percent}%` }"
              />
              <div
                v-for="mark in sliderMarks"
                :key="mark.key"
                class="memory-slider-mark"
                :class="{
                  'memory-slider-mark-left': mark.edge === 'left',
                  'memory-slider-mark-right': mark.edge === 'right'
                }"
                :style="{ left: `${mark.percent}%` }"
              >
                <span class="memory-slider-mark-tick" />
                <span class="memory-slider-mark-label">{{ mark.label }}</span>
              </div>
            </div>
          </div>
        </div>
        <div class="flex flex-col items-center pt-0.5">
          <div class="flex items-center gap-1.5">
            <Input
              class="w-20 text-right tabular-nums"
              :model-value="memoryInputValue"
              inputmode="numeric"
              @update:modelValue="updateFromInput"
            />
            <span class="text-xs text-muted-foreground tabular-nums">MB</span>
          </div>
          <span class="m-3 text-[10px] leading-none text-center text-muted-foreground/70 tabular-nums">
            1 GB = 1024 MB
          </span>
        </div>
      </div>
    </div>
    <p v-if="memoryLimitCopy" class="text-xs text-muted-foreground">{{ memoryLimitCopy }}</p>
  </div>
</template>

<style scoped>
.memory-slider {
  width: 100%;
  height: 14px;
  margin: 0;
  padding: 0;
  appearance: none;
  -webkit-appearance: none;
  background: transparent;
  outline: none;
}

.memory-slider-stack {
  flex: 1;
  min-width: 0;
}

.memory-slider-track-geometry {
  margin-inline: var(--memory-track-inset, 7px);
}

.memory-slider::-webkit-slider-runnable-track {
  height: 8px;
  border-radius: 9999px;
  background: linear-gradient(
    to right,
    hsl(var(--primary)) 0%,
    hsl(var(--primary)) var(--memory-slider-fill, 0%),
    hsl(var(--muted-foreground) / 0.35) var(--memory-slider-fill, 0%),
    hsl(var(--muted-foreground) / 0.35) 100%
  );
  transition: background 80ms linear;
}

.memory-slider::-webkit-slider-thumb {
  -webkit-appearance: none;
  appearance: none;
  width: 14px;
  height: 14px;
  margin-top: -3px;
  border-radius: 9999px;
  background: hsl(var(--primary));
  border: 2px solid hsl(var(--background));
  box-shadow: 0 0 0 2px hsl(var(--primary) / 0.25);
  cursor: pointer;
}

.memory-slider::-moz-range-track {
  height: 8px;
  border-radius: 9999px;
  background: hsl(var(--muted-foreground) / 0.35);
  transition: background 80ms linear;
}

.memory-slider::-moz-range-progress {
  height: 8px;
  border-radius: 9999px;
  background: hsl(var(--primary));
  transition: background 80ms linear;
}

.memory-slider::-moz-range-thumb {
  width: 14px;
  height: 14px;
  border-radius: 9999px;
  background: hsl(var(--primary));
  border: 2px solid hsl(var(--background));
  box-shadow: 0 0 0 2px hsl(var(--primary) / 0.25);
  cursor: pointer;
}

.memory-slider-marking-layer {
  position: relative;
  height: 24px;
}

.memory-slider-step-tick {
  position: absolute;
  top: 1px;
  width: 1px;
  height: 5px;
  border-radius: 9999px;
  background: hsl(var(--muted-foreground) / 0.28);
  transform: translateX(-50%);
}

.memory-slider-step-tick-left {
  transform: translateX(0%);
}

.memory-slider-step-tick-right {
  transform: translateX(-100%);
}

.memory-slider-mark {
  position: absolute;
  top: 1px;
  transform: translateX(-50%);
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
}

.memory-slider-mark-left {
  transform: translateX(0%);
  align-items: flex-start;
}

.memory-slider-mark-right {
  transform: translateX(-100%);
  align-items: flex-end;
}

.memory-slider-mark-tick {
  width: 2px;
  height: 8px;
  border-radius: 9999px;
  background: hsl(var(--muted-foreground) / 0.5);
}

.memory-slider-mark-label {
  font-size: 10px;
  line-height: 1;
  color: hsl(var(--muted-foreground));
  white-space: nowrap;
}
</style>
