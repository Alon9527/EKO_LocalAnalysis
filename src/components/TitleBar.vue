<script setup lang="ts">
const isTauri = !!(window as any).__TAURI_INTERNALS__;

async function minimize() {
  if (!isTauri) return;
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  await getCurrentWindow().minimize();
}

async function toggleMaximize() {
  if (!isTauri) return;
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  await getCurrentWindow().toggleMaximize();
}

async function closeWindow() {
  if (!isTauri) return;
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  await getCurrentWindow().close();
}
</script>

<template>
  <header
    data-tauri-drag-region
    class="relative z-50 flex items-center justify-end h-9 shrink-0 select-none"
  >
    <div class="flex h-full" data-tauri-drag-region="false">
      <button
        type="button"
        @click="minimize"
        class="w-11 h-9 flex items-center justify-center text-white/55 hover:text-white hover:bg-white/[0.08] transition-colors"
      >
        <svg width="11" height="11" viewBox="0 0 12 12" style="pointer-events:none"><path d="M2 6h8" stroke="currentColor" stroke-width="1.5"/></svg>
      </button>
      <button
        type="button"
        @click="toggleMaximize"
        class="w-11 h-9 flex items-center justify-center text-white/55 hover:text-white hover:bg-white/[0.08] transition-colors"
      >
        <svg width="10" height="10" viewBox="0 0 12 12" style="pointer-events:none"><rect x="2" y="2" width="8" height="8" rx="1" stroke="currentColor" stroke-width="1.2" fill="none"/></svg>
      </button>
      <button
        type="button"
        @click="closeWindow"
        class="w-11 h-9 flex items-center justify-center text-white/55 hover:text-white hover:bg-red-500/80 transition-colors"
      >
        <svg width="11" height="11" viewBox="0 0 12 12" style="pointer-events:none"><path d="M3 3l6 6M9 3L3 9" stroke="currentColor" stroke-width="1.5"/></svg>
      </button>
    </div>
  </header>
</template>
