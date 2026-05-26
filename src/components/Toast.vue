<script setup lang="ts">
import { ref } from "vue";

interface ToastItem {
  id: number;
  message: string;
  type: "success" | "error" | "info";
}

const toasts = ref<ToastItem[]>([]);
let nextId = 0;

function show(message: string, type: "success" | "error" | "info" = "info") {
  const id = nextId++;
  toasts.value.push({ id, message, type });
  setTimeout(() => {
    toasts.value = toasts.value.filter((t) => t.id !== id);
  }, 3000);
}

defineExpose({ show });

const colorMap = {
  success: "bg-emerald-400/10 border-emerald-400/20 text-emerald-400",
  error: "bg-red-400/10 border-red-400/20 text-red-400",
  info: "bg-blue-400/10 border-blue-400/20 text-blue-400",
};
</script>

<template>
  <div class="fixed top-12 right-4 z-[200] flex flex-col gap-2 pointer-events-none">
    <transition-group name="toast">
      <div
        v-for="toast in toasts"
        :key="toast.id"
        class="pointer-events-auto px-5 py-3 rounded-xl text-sm font-medium border backdrop-blur-[24px] shadow-[0_8px_16px_rgba(0,0,0,0.15),0_24px_48px_rgba(0,0,0,0.25)] max-w-[360px]"
        :class="colorMap[toast.type]"
      >
        {{ toast.message }}
      </div>
    </transition-group>
  </div>
</template>

<style scoped>
.toast-enter-active { animation: slideIn 0.3s cubic-bezier(0.16, 1, 0.3, 1); }
.toast-leave-active { transition: all 0.2s ease; }
.toast-leave-to { opacity: 0; transform: translateX(20px); }
@keyframes slideIn {
  from { opacity: 0; transform: translateX(24px); }
  to { opacity: 1; transform: translateX(0); }
}
</style>
