<script setup lang="ts">
import { onMounted } from "vue";
import { useSettingsStore } from "@/stores/settings";
import { useUpdateStore } from "@/stores/update";
import TitleBar from "@/components/TitleBar.vue";
import SideBar from "@/components/SideBar.vue";

const settingsStore = useSettingsStore();
const updateStore = useUpdateStore();

onMounted(() => {
  settingsStore.load();
  updateStore.check(true);
});
</script>

<template>
  <div class="relative flex flex-col h-screen w-screen overflow-hidden
              bg-[radial-gradient(120%_120%_at_50%_0%,#121225_0%,#0c0c14_45%,#07070c_100%)] text-white">

    <!-- 🌌 Ambient glow system -->
    <div class="pointer-events-none absolute top-[-20%] left-[-10%] w-[70%] h-[70%]
                bg-[radial-gradient(circle,rgba(45,212,191,0.12)_0%,transparent_60%)]
                blur-2xl" />

    <div class="pointer-events-none absolute bottom-[-25%] right-[-10%] w-[60%] h-[60%]
                bg-[radial-gradient(circle,rgba(99,102,241,0.10)_0%,transparent_65%)]
                blur-2xl" />

    <!-- vignette -->
    <div class="pointer-events-none absolute inset-0
                bg-[radial-gradient(circle,transparent_40%,rgba(0,0,0,0.7)_100%)]" />

    <!-- subtle noise overlay -->
    <div class="pointer-events-none absolute inset-0 opacity-[0.05] mix-blend-overlay
                bg-[url('/noise.png')]" />

    <!-- App chrome -->
    <TitleBar />

    <div class="flex flex-1 min-h-0 relative">

      <!-- Sidebar -->
      <SideBar
        class="relative z-10 backdrop-blur-xl bg-white/5 border-r border-white/10"
      />

      <!-- Main -->
      <main class="flex-1 min-w-0 relative overflow-hidden">
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
