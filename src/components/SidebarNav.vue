<script setup lang="ts">
type TabKey = "library" | "settings";

const props = defineProps<{
  activeTab: TabKey;
  status: string;
}>();

const emit = defineEmits<{
  (event: "select", value: TabKey): void;
}>();

function selectTab(tab: TabKey) {
  emit("select", tab);
}
</script>

<template>
  <aside class="glass flex h-fit flex-col gap-4 rounded-3xl border border-border/40 bg-card/60 p-4">
    <div class="space-y-1">
      <div class="text-xs uppercase tracking-[0.3em] text-muted-foreground">Navigation</div>
      <div class="text-lg font-semibold text-foreground">Atlas</div>
    </div>
    <div class="flex flex-col gap-2">
      <button
        class="group flex items-center justify-between rounded-2xl border px-4 py-3 text-left text-sm transition"
        :class="
          props.activeTab === 'library'
            ? 'border-primary/60 bg-primary/10 text-foreground'
            : 'border-border/40 bg-secondary/40 text-muted-foreground hover:text-foreground'
        "
        @click="selectTab('library')"
      >
        <span class="font-semibold">Library</span>
        <span class="text-xs uppercase tracking-widest opacity-60">Games</span>
      </button>
      <button
        class="group flex items-center justify-between rounded-2xl border px-4 py-3 text-left text-sm transition"
        :class="
          props.activeTab === 'settings'
            ? 'border-primary/60 bg-primary/10 text-foreground'
            : 'border-border/40 bg-secondary/40 text-muted-foreground hover:text-foreground'
        "
        @click="selectTab('settings')"
      >
        <span class="font-semibold">Settings</span>
        <span class="text-xs uppercase tracking-widest opacity-60">Config</span>
      </button>
    </div>
    <div class="mt-auto rounded-2xl border border-border/40 bg-secondary/40 px-4 py-3 text-xs text-muted-foreground">
      <div class="text-xs uppercase tracking-widest">Status</div>
      <div class="mt-2 font-semibold text-foreground">{{ props.status }}</div>
    </div>
  </aside>
</template>
