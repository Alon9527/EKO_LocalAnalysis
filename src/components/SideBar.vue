<script setup lang="ts">
import { useRoute, useRouter } from "vue-router";
import { Picture, Files, Clock, Download, Setting } from "@element-plus/icons-vue";

const route = useRoute();
const router = useRouter();

const navItems = [
  { path: "/single", label: "单图分析", icon: Picture },
  { path: "/batch", label: "批量分析", icon: Files },
  { path: "/gallery", label: "历史记录", icon: Clock },
  { path: "/export", label: "导出结果", icon: Download },
  { path: "/settings", label: "设置中心", icon: Setting },
];

function handleSelect(index: string) {
  router.push(index);
}
</script>

<template>
  <aside class="w-[260px] shrink-0 flex flex-col select-none">
    <!-- Logo -->
    <div data-tauri-drag-region class="flex items-center gap-3 px-5 h-20 shrink-0">
      <div class="w-12 h-12 rounded-xl bg-black border border-white/15 flex items-center justify-center shrink-0 shadow-lg">
        <svg width="34" height="14" viewBox="0 0 34 14" fill="none" xmlns="http://www.w3.org/2000/svg">
          <!-- E -->
          <path d="M0 0h9v2.4H2.4v3h5.4V7.8H2.4v3.8H9V14H0V0z" fill="white" />
          <!-- K -->
          <path d="M12 0h2.4v6L20 0h3l-5.5 6L23 14h-3l-5.6-6v6H12V0z" fill="white" />
          <!-- O -->
          <path d="M26 0h7a1 1 0 011 1v12a1 1 0 01-1 1h-7a1 1 0 01-1-1V1a1 1 0 011-1zm.4 2.4v9.2h6.2V2.4h-6.2z" fill="white" />
        </svg>
      </div>
      <span class="text-[17px] font-semibold text-white/95 tracking-wide">图片反推工具</span>
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
  </aside>
</template>

<style>
.app-sidebar-menu.el-menu {
  border-right: none !important;
  padding: 12px 16px;
}
.app-sidebar-menu .el-menu-item {
  height: 56px !important;
  line-height: 56px !important;
  border-radius: 14px !important;
  margin-bottom: 6px;
  font-size: 16px !important;
  font-weight: 500 !important;
  padding: 0 18px !important;
}
.app-sidebar-menu .el-menu-item:hover {
  background-color: rgba(255, 255, 255, 0.05) !important;
  color: rgba(255, 255, 255, 0.9) !important;
}
.app-sidebar-menu .el-menu-item.is-active {
  background-color: rgba(45, 212, 191, 0.13) !important;
  position: relative;
}
.app-sidebar-menu .el-menu-item.is-active::before {
  content: "";
  position: absolute;
  left: -4px;
  top: 50%;
  transform: translateY(-50%);
  width: 3px;
  height: 22px;
  border-radius: 2px;
  background: #2dd4bf;
  box-shadow: 0 0 12px rgba(45, 212, 191, 0.7);
}
.app-sidebar-menu .el-menu-item .el-icon {
  margin-right: 14px !important;
  width: 22px;
  font-size: 22px !important;
}
</style>
