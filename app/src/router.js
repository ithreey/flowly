import { createRouter, createWebHashHistory } from "vue-router";
import Monitor from "./pages/Monitor.vue";
import Rules from "./pages/Rules.vue";
import Certificates from "./pages/Certificates.vue";
import Settings from "./pages/Settings.vue";
import AppSettings from "./pages/AppSettings.vue";

const router = createRouter({
  // Tauri 内嵌前端走 file:// 协议，用 hash history 避免刷新 404。
  history: createWebHashHistory(),
  routes: [
    { path: "/", redirect: "/monitor" },
    { path: "/monitor", component: Monitor, meta: { title: "流量监控" } },
    { path: "/rules", component: Rules, meta: { title: "规则配置" } },
    { path: "/certs", component: Certificates, meta: { title: "证书管理" } },
    { path: "/settings", component: Settings, meta: { title: "代理设置" } },
    {
      path: "/app-settings",
      component: AppSettings,
      meta: { title: "应用设置" },
    },
  ],
});

export default router;
