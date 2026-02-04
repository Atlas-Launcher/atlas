<script setup lang="ts">
import { computed, useAttrs } from "vue";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "../../../lib/utils";

const buttonVariants = cva(
  "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50",
  {
    variants: {
      variant: {
        default: "bg-primary text-primary-foreground hover:bg-primary/90",
        secondary: "bg-secondary text-secondary-foreground hover:bg-secondary/80",
        ghost: "hover:bg-white/10 text-foreground",
        outline: "border border-input bg-transparent hover:bg-white/10"
      },
      size: {
        sm: "h-9 px-3",
        md: "h-10 px-4",
        lg: "h-11 px-6"
      }
    },
    defaultVariants: {
      variant: "default",
      size: "md"
    }
  }
);

type ButtonVariants = VariantProps<typeof buttonVariants>;

const props = withDefaults(
  defineProps<{
    variant?: ButtonVariants["variant"];
    size?: ButtonVariants["size"];
  }>(),
  {
    variant: "default",
    size: "md"
  }
);

const attrs = useAttrs();
const classes = computed(() => cn(buttonVariants({ variant: props.variant, size: props.size }), attrs.class));
</script>

<template>
  <button v-bind="attrs" :class="classes">
    <slot />
  </button>
</template>
