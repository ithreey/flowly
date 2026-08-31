<template>
  <el-container class="layout">
    <el-aside width="236px" class="sidebar">
      <div class="brand">
        <div class="brand-mark">FL</div>
        <div class="brand-copy">
          <div class="brand-title">Flowly</div>
          <div class="brand-subtitle">Proxy Inspection Console</div>
        </div>
      </div>
      <el-menu :default-active="active" router class="menu">
        <el-menu-item index="/monitor">
          <el-icon><Odometer /></el-icon>
          <span>流量监控</span>
        </el-menu-item>
        <el-menu-item index="/rules">
          <el-icon><Document /></el-icon>
          <span>规则配置</span>
        </el-menu-item>
        <el-menu-item index="/certs">
          <el-icon><Lock /></el-icon>
          <span>证书管理</span>
        </el-menu-item>
        <el-menu-item index="/settings">
          <el-icon><Setting /></el-icon>
          <span>代理设置</span>
        </el-menu-item>
        <el-menu-item index="/app-settings">
          <el-icon><Tools /></el-icon>
          <span>应用设置</span>
        </el-menu-item>
        <el-menu-item index="/sender">
          <el-icon><Promotion /></el-icon>
          <span>发送器</span>
        </el-menu-item>
      </el-menu>
      <div class="runtime-card">
        <div class="runtime-label">监听地址</div>
        <div class="runtime-row">
          <span v-if="localIp" class="runtime-ip">{{ localIp }}:{{ runtimePort }}</span>
          <span v-else>{{ runtimePort }}</span>
          <span class="status-dot" :class="{ stopped: !proxyStatus.running }" />
        </div>
      </div>
    </el-aside>
    <el-container class="workspace">
      <el-header class="topbar" height="74px">
        <div class="page-title">
          <h1>{{ pageTitle }}</h1>
          <p>{{ pageDescription }}</p>
        </div>
        <div class="top-actions">
          <div class="status-pill" :class="{ stopped: !proxyStatus.running }">
            <span
              class="status-dot"
              :class="{ stopped: !proxyStatus.running }"
            />
            {{ proxyStatus.running ? "代理运行中" : "代理已停止" }}
          </div>
          <el-button
            v-if="!proxyStatus.running"
            type="primary"
            :loading="proxyBusy"
            @click="startProxy"
          >
            启动代理
          </el-button>
          <el-button
            v-else
            type="danger"
            :loading="proxyBusy"
            @click="stopProxy"
          >
            停止代理
          </el-button>
        </div>
      </el-header>
      <el-main class="main">
        <router-view />
      </el-main>
    </el-container>

    <!-- 全局拦截弹窗 -->
    <InterceptModal />
  </el-container>
</template>

<script setup>
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useRoute } from "vue-router";
import { ElMessage } from "element-plus";
import { invoke } from "@tauri-apps/api/core";
import {
  Document,
  Lock,
  Odometer,
  Promotion,
  Setting,
  Tools,
} from "@element-plus/icons-vue";
import InterceptModal from "./components/InterceptModal.vue";

const route = useRoute();
const active = computed(() => route.path);
const runtimePort = computed(() => {
  const listenAddr = String(proxyStatus.value.listenAddr || "34567");
  return listenAddr.includes(":") ? listenAddr.split(":").pop() : listenAddr;
});
const localIp = ref("");
const proxyBusy = ref(false);
const proxyStatus = ref({
  running: false,
  listenAddr: "",
  upstreamProxy: null,
});
let statusTimer = null;

async function fetchLocalIp() {
  try {
    localIp.value = await invoke("get_local_ipv4");
  } catch {
    localIp.value = "";
  }
}

const pageDescriptions = {
  "/monitor": "捕获 HTTP/HTTPS 会话，筛选、检查并导出 HAR 数据。",
  "/sender": "构造和发送 HTTP 请求，调试 API 接口。",
  "/rules": "管理实时生效的流量匹配、过滤和修改规则。",
  "/certs": "生成、安装和复制用于 HTTPS 解密的 CA 证书。",
  "/settings": "配置监听端口、上游代理和系统代理接管策略。",
  "/app-settings": "调整应用级显示和工作台偏好。",
};

const pageTitle = computed(() => route.meta?.title || "Flowly");
const pageDescription = computed(
  () => pageDescriptions[route.path] || "MITM 代理调试工作台。",
);

async function refreshProxyStatus() {
  try {
    proxyStatus.value = await invoke("proxy_status");
  } catch {
    proxyStatus.value = { running: false, listenAddr: "", upstreamProxy: null };
  }
}

async function startProxy() {
  proxyBusy.value = true;
  try {
    const cfg = await invoke("config_get");
    await invoke("proxy_start", {
      listenAddr: cfg.listenAddr,
      upstreamProxy: cfg.upstreamProxy || null,
    });
    await refreshProxyStatus();
    ElMessage.success("代理已启动");
  } catch (e) {
    ElMessage.error(String(e));
  } finally {
    proxyBusy.value = false;
  }
}

async function stopProxy() {
  proxyBusy.value = true;
  try {
    await invoke("proxy_stop");
    await refreshProxyStatus();
    ElMessage.success("代理已停止");
  } catch (e) {
    ElMessage.error(String(e));
  } finally {
    proxyBusy.value = false;
  }
}

watch(
  () => route.path,
  () => refreshProxyStatus(),
);

onMounted(() => {
  refreshProxyStatus();
  fetchLocalIp();
  statusTimer = window.setInterval(refreshProxyStatus, 4000);
});

onBeforeUnmount(() => {
  if (statusTimer) window.clearInterval(statusTimer);
});
</script>

<style>
:root {
  color-scheme: dark;
  --app-font-size: 14px;
  --el-component-size-small: 28px;
  --el-component-size: 32px;
  --el-component-size-large: 36px;
  --gm-bg: #08111f;
  --gm-bg-deep: #0a1020;
  --gm-panel: #0f1b2d;
  --gm-panel-2: #121f33;
  --gm-panel-3: #17263c;
  --gm-line: rgba(148, 163, 184, 0.18);
  --gm-line-strong: rgba(56, 189, 248, 0.34);
  --gm-text: #e5edf8;
  --gm-muted: #8fa1b7;
  --gm-subtle: #65748a;
  --gm-green: #22c55e;
  --gm-cyan: #38bdf8;
  --gm-blue: #3b82f6;
  --gm-amber: #f59e0b;
  --gm-red: #f87171;
  --gm-purple: #a78bfa;
  --gm-code: #07101d;
  --gm-shadow: 0 24px 70px rgba(0, 0, 0, 0.34);
  --el-color-primary: #38bdf8;
  --el-color-success: #22c55e;
  --el-color-warning: #f59e0b;
  --el-color-danger: #f87171;
  --el-text-color-primary: var(--gm-text);
  --el-text-color-regular: var(--gm-muted);
  --el-text-color-secondary: var(--gm-subtle);
  --el-border-color: var(--gm-line);
  --el-border-color-light: rgba(148, 163, 184, 0.12);
  --el-fill-color-blank: rgba(8, 17, 31, 0.72);
  --el-fill-color-light: rgba(18, 31, 51, 0.72);
  --el-bg-color: var(--gm-panel);
  --el-bg-color-overlay: var(--gm-panel-2);
}

body {
  margin: 0;
  font-family: system-ui, "Segoe UI", "Microsoft YaHei", sans-serif;
  font-size: var(--app-font-size);
  background:
    radial-gradient(
      circle at 18% 12%,
      rgba(56, 189, 248, 0.16),
      transparent 32%
    ),
    linear-gradient(
      135deg,
      var(--gm-bg) 0%,
      var(--gm-bg-deep) 52%,
      #0b1625 100%
    );
  color: var(--gm-text);
}

body::before {
  position: fixed;
  inset: 0;
  content: "";
  pointer-events: none;
  background-image:
    linear-gradient(rgba(148, 163, 184, 0.055) 1px, transparent 1px),
    linear-gradient(90deg, rgba(148, 163, 184, 0.045) 1px, transparent 1px);
  background-size: 42px 42px;
  mask-image: linear-gradient(to bottom, rgba(0, 0, 0, 0.72), transparent 82%);
}

.layout {
  height: 100vh;
  background: transparent;
}

.sidebar {
  display: flex;
  flex-direction: column;
  gap: 20px;
  padding: 18px 14px;
  border-right: 1px solid var(--gm-line);
  background: rgba(8, 17, 31, 0.78);
  backdrop-filter: blur(18px);
}

.brand {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 8px 14px;
  border-bottom: 1px solid var(--gm-line);
}

.brand-mark {
  display: grid;
  width: 38px;
  height: 38px;
  place-items: center;
  border: 1px solid rgba(56, 189, 248, 0.48);
  border-radius: 10px;
  background: linear-gradient(
    145deg,
    rgba(56, 189, 248, 0.22),
    rgba(34, 197, 94, 0.12)
  );
  box-shadow: inset 0 0 24px rgba(56, 189, 248, 0.12);
  color: var(--gm-cyan);
  font-family: "Cascadia Mono", "JetBrains Mono", Consolas, monospace;
  font-weight: 700;
}

.brand-title {
  font-size: 16px;
  font-weight: 700;
}

.brand-subtitle {
  margin-top: 2px;
  color: var(--gm-muted);
  font-size: 12px;
}

.menu {
  border-right: none;
  background: transparent;
}

.menu .el-menu-item {
  height: 44px;
  margin-bottom: 6px;
  border: 1px solid transparent;
  border-radius: 8px;
  color: var(--gm-muted);
}

.menu .el-menu-item:hover {
  border-color: rgba(56, 189, 248, 0.26);
  background: rgba(56, 189, 248, 0.08);
  color: var(--gm-text);
}

.menu .el-menu-item.is-active {
  border-color: var(--gm-line-strong);
  background: rgba(56, 189, 248, 0.11);
  box-shadow: inset 3px 0 0 var(--gm-cyan);
  color: var(--gm-text);
}

.runtime-card {
  margin-top: auto;
  padding: 12px;
  border: 1px solid var(--gm-line);
  border-radius: 8px;
  background: rgba(15, 27, 45, 0.64);
}

.runtime-label {
  color: var(--gm-subtle);
  font-size: 11px;
  text-transform: uppercase;
}

.runtime-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  margin-top: 8px;
  color: var(--gm-text);
  font-family: "Cascadia Mono", "JetBrains Mono", Consolas, monospace;
  font-size: 12px;
}

.runtime-ip {
  color: var(--gm-muted);
}

.status-dot {
  display: inline-block;
  width: 8px;
  height: 8px;
  flex: 0 0 auto;
  border-radius: 50%;
  background: var(--gm-green);
  box-shadow: 0 0 16px rgba(34, 197, 94, 0.7);
}

.status-dot.stopped {
  background: var(--gm-subtle);
  box-shadow: none;
}

.workspace {
  min-width: 0;
  background: transparent;
}

.topbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 18px;
  padding: 16px 22px;
  border-bottom: 1px solid var(--gm-line);
  background: rgba(11, 22, 37, 0.66);
  backdrop-filter: blur(18px);
}

.page-title h1 {
  margin: 0;
  color: var(--gm-text);
  font-size: 20px;
  font-weight: 700;
}

.page-title p {
  margin: 4px 0 0;
  color: var(--gm-muted);
  font-size: 12px;
}

.top-actions {
  display: flex;
  align-items: center;
  gap: 10px;
}

.status-pill {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  height: 34px;
  padding: 0 12px;
  border: 1px solid rgba(34, 197, 94, 0.32);
  border-radius: 7px;
  background: rgba(34, 197, 94, 0.1);
  color: #b7f7c9;
  white-space: nowrap;
}

.status-pill.stopped,
.status-pill.muted {
  border-color: var(--gm-line);
  background: rgba(15, 27, 45, 0.9);
  color: var(--gm-muted);
}

.main {
  padding: 16px;
  overflow: auto;
  min-height: 0;
  background: transparent;
}

.el-button {
  box-sizing: border-box;
  height: 34px;
  min-height: 34px;
  padding-top: 0;
  padding-bottom: 0;
  border-color: var(--gm-line);
  background: rgba(15, 27, 45, 0.9);
  color: var(--gm-text);
  line-height: 32px;
  transition:
    border-color 180ms ease,
    background 180ms ease,
    color 180ms ease;
}

.el-button:hover,
.el-button:focus {
  border-color: rgba(56, 189, 248, 0.55);
  background: rgba(56, 189, 248, 0.14);
  color: var(--gm-text);
}

.el-button--primary {
  border-color: rgba(34, 197, 94, 0.45);
  background: linear-gradient(
    180deg,
    rgba(34, 197, 94, 0.94),
    rgba(21, 128, 61, 0.92)
  );
  color: #04140a;
  font-weight: 700;
}

.el-button--danger {
  border-color: rgba(248, 113, 113, 0.42);
  background: rgba(248, 113, 113, 0.12);
  color: #fecaca;
}

.el-button.is-disabled,
.el-button.is-disabled:hover,
.el-button.is-disabled:focus {
  border-color: rgba(148, 163, 184, 0.14);
  background: rgba(15, 27, 45, 0.42);
  color: var(--gm-subtle);
  cursor: not-allowed;
}

.el-input__wrapper,
.el-select__wrapper {
  box-sizing: border-box;
  min-height: 34px;
  border: 1px solid var(--gm-line);
  border-radius: 8px;
  background: rgba(8, 17, 31, 0.72);
  box-shadow: none;
}

.el-input__wrapper.is-focus,
.el-select__wrapper.is-focused {
  border-color: var(--gm-cyan);
  box-shadow: 0 0 0 3px rgba(56, 189, 248, 0.12);
}

.el-select__wrapper.is-filterable {
  min-height: 34px;
}

.el-select__wrapper.is-filterable.is-multiple {
  padding-top: 2px;
  padding-bottom: 2px;
}

.el-input__inner {
  color: var(--gm-text);
  min-height: 30px;
  line-height: 30px;
}

.el-input__inner::placeholder {
  color: var(--gm-subtle);
}

.el-textarea__inner::placeholder {
  color: rgba(101, 116, 138, 0.5);
}

/* CodeMirror 行号区域适配深色主题 */
.cm-gutters {
  background: rgba(15, 27, 45, 0.9) !important;
  border-right: 1px solid var(--gm-line) !important;
}
.cm-lineNumbers .cm-activeLineGutter {
  background: rgba(56, 189, 248, 0.1) !important;
}
.cm-lineNumbers {
  color: var(--gm-subtle) !important;
}

.el-select__selection {
  min-height: 22px;
}

.el-select__selected-item {
  height: 22px;
  line-height: 22px;
}

.el-radio-button__inner {
  border-color: var(--gm-line);
  background: rgba(8, 17, 31, 0.72);
  color: var(--gm-muted);
  min-height: 30px;
  line-height: 28px;
}

.el-tag {
  box-sizing: border-box;
  border-color: var(--gm-line);
  background: rgba(8, 17, 31, 0.56);
  color: var(--gm-muted);
  height: 28px;
  min-height: 28px;
  padding-top: 0;
  padding-bottom: 0;
  line-height: 26px;
  font-size: var(--app-font-size);
}

.el-select__selected-item .el-tag {
  height: 22px;
  min-height: 22px;
  line-height: 20px;
  margin: 0 2px;
}

.el-select__selected-item .el-tag__close {
  top: 0;
}

.el-table {
  font-size: var(--app-font-size);
  --el-table-bg-color: transparent;
  --el-table-tr-bg-color: transparent;
  --el-table-header-bg-color: rgba(12, 24, 39, 0.96);
  --el-table-header-text-color: var(--gm-subtle);
  --el-table-text-color: var(--gm-text);
  --el-table-border-color: rgba(148, 163, 184, 0.1);
  --el-table-row-hover-bg-color: rgba(56, 189, 248, 0.07);
  background: transparent;
}

.el-table th,
.el-table td,
.el-table .cell {
  font-size: inherit;
}

.el-card,
.el-dialog,
.el-drawer {
  border-color: var(--gm-line);
  background: var(--gm-panel);
  color: var(--gm-text);
}

.el-card__header,
.el-dialog__header,
.el-drawer__header {
  border-bottom-color: var(--gm-line);
  color: var(--gm-text);
}

@media (max-width: 1180px) {
  .sidebar {
    width: 78px !important;
    padding: 18px 10px;
  }

  .brand-copy,
  .menu span,
  .runtime-card {
    display: none;
  }

  .brand,
  .menu .el-menu-item {
    justify-content: center;
  }

  .topbar {
    align-items: flex-start;
    height: auto !important;
    min-height: 74px;
    flex-direction: column;
  }

  .top-actions {
    flex-wrap: wrap;
  }
}
</style>
