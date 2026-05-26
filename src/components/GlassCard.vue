<script setup lang="ts">
import { cn } from "@/lib/utils";

defineProps<{ class?: string }>();
</script>

<template>
  <div :class="cn('bg-white/[0.06] border border-white/[0.1] rounded-2xl backdrop-blur-2xl shadow-[0_4px_24px_rgba(0,0,0,0.3),inset_0_1px_0_rgba(255,255,255,0.06)] transition-all duration-300', $props.class)">
    <slot />
  </div>
</template>
