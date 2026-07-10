<script setup lang="ts">
import { useRoute, useRouter } from "vue-router";
import { Picture, Files, Clock, Download, Setting, Refresh } from "@element-plus/icons-vue";
import { useUpdateStore } from "@/stores/update";

const route = useRoute();
const router = useRouter();
const updateStore = useUpdateStore();

const navItems = [
  { path: "/single", label: "单图分析", icon: Picture },
  { path: "/batch", label: "批量分析", icon: Files },
  { path: "/gallery", label: "历史记录", icon: Clock },
  { path: "/export", label: "结果管理", icon: Download },
  { path: "/settings", label: "设置中心", icon: Setting },
];

function handleSelect(index: string) {
  router.push(index);
}

function openUpdate() {
  router.push("/settings");
}
</script>

<template>
  <aside class="w-[232px] shrink-0 flex flex-col select-none">
    <!-- Logo -->
    <div data-tauri-drag-region class="flex items-center gap-3 px-4 h-[72px] shrink-0">
      <div class="w-11 h-11 rounded-xl bg-black border border-white/12 flex items-center justify-center shrink-0 shadow-sm">
        <svg width="32" height="13" viewBox="0 0 34 14" fill="none" xmlns="http://www.w3.org/2000/svg">
          <!-- E -->
          <path d="M0 0h9v2.4H2.4v3h5.4V7.8H2.4v3.8H9V14H0V0z" fill="white" />
          <!-- K -->
          <path d="M12 0h2.4v6L20 0h3l-5.5 6L23 14h-3l-5.6-6v6H12V0z" fill="white" />
          <!-- O -->
          <path d="M26 0h7a1 1 0 011 1v12a1 1 0 01-1 1h-7a1 1 0 01-1-1V1a1 1 0 011-1zm.4 2.4v9.2h6.2V2.4h-6.2z" fill="white" />
        </svg>
      </div>
      <span class="text-[16px] font-semibold text-white/92 tracking-wide">图片反推工具</span>
    </div>

    <!-- Menu -->
    <el-menu
      :default-active="route.path"
      class="app-sidebar-menu"
      background-color="transparent"
      text-color="rgba(255,255,255,0.6)"
      active-text-color="#2dd4bf"
      @select="handleSelect"
    >
      <el-menu-item v-for="item in navItems" :key="item.path" :index="item.path">
        <el-icon :size="20"><component :is="item.icon" /></el-icon>
        <template #title>{{ item.label }}</template>
      </el-menu-item>
    </el-menu>

    <div class="mt-auto px-4 pb-5">
      <button
        v-if="updateStore.updateAvailable"
        class="update-notice"
        type="button"
        @click="openUpdate"
      >
        <span class="update-notice__icon">
          <el-icon :size="16"><Refresh /></el-icon>
        </span>
        <span class="min-w-0 text-left">
          <span class="block text-[13px] font-semibold leading-5">发现新版本</span>
          <span class="block text-[12px] text-teal-100/70 truncate">v{{ updateStore.version }} 可更新</span>
        </span>
      </button>
    </div>
  </aside>
</template>

<style>
.app-sidebar-menu.el-menu {
  border-right: none !important;
  padding: 8px 12px;
}
.app-sidebar-menu .el-menu-item {
  height: 48px !important;
  line-height: 48px !important;
  border-radius: 12px !important;
  margin-bottom: 4px;
  font-size: 14px !important;
  font-weight: 500 !important;
  padding: 0 14px !important;
}
.app-sidebar-menu .el-menu-item:hover {
  background-color: rgba(255, 255, 255, 0.05) !important;
  color: rgba(255, 255, 255, 0.9) !important;
}
.app-sidebar-menu .el-menu-item.is-active {
  background-color: rgba(45, 212, 191, 0.11) !important;
  position: relative;
}
.app-sidebar-menu .el-menu-item.is-active::before {
  content: "";
  position: absolute;
  left: -4px;
  top: 50%;
  transform: translateY(-50%);
  width: 3px;
  height: 20px;
  border-radius: 2px;
  background: #2dd4bf;
  box-shadow: 0 0 10px rgba(45, 212, 191, 0.42);
}
.app-sidebar-menu .el-menu-item .el-icon {
  margin-right: 12px !important;
  width: 20px;
  font-size: 20px !important;
}
.update-notice {
  display: flex;
  width: 100%;
  align-items: center;
  gap: 10px;
  border-radius: 12px;
  border: 1px solid rgba(45, 212, 191, 0.32);
  background: rgba(45, 212, 191, 0.13);
  padding: 10px;
  color: rgba(255, 255, 255, 0.92);
  cursor: pointer;
  transition: all 180ms ease;
}
.update-notice:hover {
  border-color: rgba(45, 212, 191, 0.55);
  background: rgba(45, 212, 191, 0.18);
  transform: translateY(-1px);
}
.update-notice__icon {
  display: inline-flex;
  width: 28px;
  height: 28px;
  flex-shrink: 0;
  align-items: center;
  justify-content: center;
  border-radius: 9px;
  background: rgba(45, 212, 191, 0.18);
  color: #2dd4bf;
}
</style>
