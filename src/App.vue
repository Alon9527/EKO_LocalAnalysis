<script setup lang="ts">
import { onMounted } from "vue";
import { useSettingsStore } from "@/stores/settings";
import TitleBar from "@/components/TitleBar.vue";
import SideBar from "@/components/SideBar.vue";

const settingsStore = useSettingsStore();

onMounted(() => {
  settingsStore.load();
});
</script>

<template>
  <div class="relative flex flex-col h-screen w-screen overflow-hidden
              bg-[linear-gradient(145deg,#0b0d11_0%,#101117_45%,#07080b_100%)] text-white">

    <!-- Workspace texture -->
    <div class="pointer-events-none absolute inset-0
                bg-[linear-gradient(180deg,rgba(255,255,255,0.035)_0%,transparent_34%),linear-gradient(90deg,rgba(45,212,191,0.045)_0%,transparent_28%,rgba(255,255,255,0.018)_100%)]" />

    <!-- subtle noise overlay -->
    <div class="pointer-events-none absolute inset-0 opacity-[0.035] mix-blend-overlay
                bg-[url('/noise.png')]" />

    <!-- App chrome -->
    <TitleBar />

    <div class="flex flex-1 min-h-0 relative">

      <!-- Sidebar -->
      <SideBar
        class="relative z-10 bg-[#0d1014]/95 border-r border-white/[0.08]"
      />

      <!-- Main -->
      <main class="flex-1 min-w-0 relative overflow-hidden bg-[#090a0f]/45">
        <router-view v-slot="{ Component }">
          <transition name="fade" mode="out-in">
            <keep-alive>
              <component :is="Component" class="absolute inset-0 overflow-y-auto overflow-x-hidden z-[1]" />
            </keep-alive>
          </transition>
        </router-view>
      </main>
    </div>
  </div>
</template>

<style scoped>
.fade-enter-active,
.fade-leave-active {
  transition: all 260ms cubic-bezier(0.22, 1, 0.36, 1);
}

.fade-enter-from {
  opacity: 0;
  transform: translateY(8px);
}

.fade-leave-to {
  opacity: 0;
  transform: translateY(-6px);
}
</style>
