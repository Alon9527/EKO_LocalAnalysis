import { createRouter, createWebHashHistory } from "vue-router";

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: "/", redirect: "/single" },
    { path: "/single", name: "single", component: () => import("@/views/SingleView.vue") },
    { path: "/batch", name: "batch", component: () => import("@/views/BatchView.vue") },
    { path: "/gallery", name: "gallery", component: () => import("@/views/GalleryView.vue") },
    { path: "/export", name: "export", component: () => import("@/views/GalleryView.vue") },
    { path: "/settings", name: "settings", component: () => import("@/views/SettingsView.vue") },
  ],
});

export default router;
