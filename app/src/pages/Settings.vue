<template>
  <div class="settings-page">
    <section class="settings-panel">
      <div class="panel-head">
        <div>
          <div class="panel-title">代理设置</div>
          <div class="panel-subtitle">
            配置监听端口、上游代理和系统代理接管策略。
          </div>
        </div>
      </div>

      <el-form label-width="140px" @submit.prevent>
        <el-form-item label="监听端口">
          <el-input-number
            v-model="listenPort"
            :min="1"
            :max="65535"
            :step="1"
            controls-position="right"
          />
        </el-form-item>
        <el-form-item label="上游代理">
          <el-input
            v-model="upstream"
            placeholder="可选，如 http://127.0.0.1:7890"
          />
        </el-form-item>
        <el-form-item label="自动设置系统代理">
          <el-switch v-model="autoSystemProxy" />
          <span class="tip"
            >启动代理时自动将本机系统代理指向 127.0.0.1:端口，停止时还原</span
          >
        </el-form-item>
        <el-form-item>
          <el-button
            v-if="!running"
            type="primary"
            :loading="busy"
            @click="start"
          >
            启动
          </el-button>
          <el-button v-else type="danger" :loading="busy" @click="stop">
            停止
          </el-button>
        </el-form-item>
      </el-form>

      <el-alert
        v-if="error"
        :title="error"
        type="error"
        :closable="false"
        show-icon
      />

      <el-divider />

      <div class="status-grid">
        <div class="status-card">
          <div class="status-label">运行状态</div>
          <el-tag :type="running ? 'success' : 'info'" size="small">
            {{ running ? "运行中" : "已停止" }}
          </el-tag>
        </div>
        <div class="status-card">
          <div class="status-label">监听地址</div>
          <div class="status-value">{{ displayListenAddr }}</div>
        </div>
        <div class="status-card">
          <div class="status-label">上游代理</div>
          <div class="status-value">{{ status.upstreamProxy || "-" }}</div>
        </div>
      </div>
    </section>
  </div>
</template>

<script setup>
import { computed, onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

const listenPort = ref(34567);
const upstream = ref("");
const autoSystemProxy = ref(true);
const running = ref(false);
const busy = ref(false);
const error = ref("");
const localIp = ref("");
const status = ref({ running: false, listenAddr: "", upstreamProxy: null });

const displayListenAddr = computed(() => {
  const addr = status.value.listenAddr || "";
  if (!addr) return "-";
  if (!localIp.value) return addr;
  return addr.replace("0.0.0.0", localIp.value);
});

async function refresh() {
  try {
    status.value = await invoke("proxy_status");
    running.value = status.value.running;
  } catch (e) {
    error.value = String(e);
  }
}

async function fetchLocalIp() {
  try {
    localIp.value = await invoke("get_local_ipv4");
  } catch {
    localIp.value = "";
  }
}

async function loadConfig() {
  const cfg = await invoke("config_get");
  listenPort.value = portFromListenAddr(cfg.listenAddr);
  upstream.value = cfg.upstreamProxy || "";
  autoSystemProxy.value = cfg.autoSystemProxy;
  return cfg;
}

function listenAddrFromPort() {
  return `0.0.0.0:${listenPort.value || 34567}`;
}

function portFromListenAddr(listenAddr) {
  const value = String(listenAddr || "");
  const port = Number(value.includes(":") ? value.split(":").pop() : value);
  return Number.isInteger(port) && port > 0 && port <= 65535 ? port : 34567;
}

async function start() {
  error.value = "";
  busy.value = true;
  try {
    const cfg = await invoke("config_get");

    await invoke("config_set", {
      config: {
        listenAddr: listenAddrFromPort(),
        upstreamProxy: upstream.value || null,
        captureBody: cfg.captureBody ?? true,
        maxBodySize: cfg.maxBodySize,
        autoSystemProxy: autoSystemProxy.value,
        mitmHosts: cfg.mitmHosts || [],
      },
    });

    status.value = await invoke("proxy_start", {
      listenAddr: listenAddrFromPort(),
      upstreamProxy: upstream.value || null,
    });
    running.value = true;
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = false;
  }
}

async function stop() {
  error.value = "";
  busy.value = true;
  try {
    await invoke("proxy_stop");
    running.value = false;
    status.value = { running: false, listenAddr: "", upstreamProxy: null };
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = false;
  }
}

onMounted(async () => {
  try {
    await loadConfig();
  } catch (e) {
    error.value = String(e);
  }
  await refresh();
  await fetchLocalIp();
});
</script>

<style scoped>
.settings-page {
  height: 100%;
}

.settings-panel {
  max-width: 920px;
  border: 1px solid var(--gm-line);
  border-radius: 10px;
  background: rgba(15, 27, 45, 0.78);
  box-shadow: var(--gm-shadow);
}

.panel-head {
  padding: 14px 16px;
  border-bottom: 1px solid var(--gm-line);
}

.panel-title {
  color: var(--gm-text);
  font-size: 15px;
  font-weight: 700;
}

.panel-subtitle {
  margin-top: 4px;
  color: var(--gm-muted);
  font-size: 12px;
}

:deep(.el-form) {
  padding: 16px;
}

.tip {
  margin-left: 8px;
  color: var(--gm-muted);
  font-size: 12px;
  white-space: nowrap;
}

.status-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 10px;
  padding: 0 16px 16px;
}

.status-card {
  min-width: 0;
  padding: 12px;
  border: 1px solid var(--gm-line);
  border-radius: 8px;
  background: rgba(8, 17, 31, 0.42);
}

.status-label {
  margin-bottom: 8px;
  color: var(--gm-subtle);
  font-size: 11px;
  text-transform: uppercase;
}

.status-value {
  overflow: hidden;
  color: var(--gm-text);
  font-family: "Cascadia Mono", "JetBrains Mono", Consolas, monospace;
  text-overflow: ellipsis;
  white-space: nowrap;
}

:deep(.el-form-item__label) {
  white-space: nowrap;
  color: var(--gm-muted);
}

:deep(.el-form-item__content) {
  flex-wrap: nowrap;
}

@media (max-width: 900px) {
  .status-grid {
    grid-template-columns: 1fr;
  }
}
</style>
