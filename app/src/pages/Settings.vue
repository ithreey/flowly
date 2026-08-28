<template>
  <div class="settings-page">
    <section class="settings-panel">
      <div class="panel-head">
        <div>
          <div class="panel-title">代理设置</div>
          <div class="panel-subtitle">
            配置监听地址、上游代理和系统代理接管策略。
          </div>
        </div>
      </div>

      <el-form label-width="140px" @submit.prevent>
        <el-form-item label="监听地址">
          <el-input v-model="listenAddr" placeholder="127.0.0.1:34567" />
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
            >启动代理时自动将系统代理指向监听地址，停止时还原</span
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
          <div class="status-value">{{ status.listenAddr || "-" }}</div>
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
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";

const listenAddr = ref("127.0.0.1:34567");
const upstream = ref("");
const autoSystemProxy = ref(true);
const running = ref(false);
const busy = ref(false);
const error = ref("");
const status = ref({ running: false, listenAddr: "", upstreamProxy: null });

async function refresh() {
  try {
    status.value = await invoke("proxy_status");
    running.value = status.value.running;
  } catch (e) {
    error.value = String(e);
  }
}

async function loadConfig() {
  const cfg = await invoke("config_get");
  listenAddr.value = cfg.listenAddr;
  upstream.value = cfg.upstreamProxy || "";
  autoSystemProxy.value = cfg.autoSystemProxy;
  return cfg;
}

async function start() {
  error.value = "";
  busy.value = true;
  try {
    const cfg = await invoke("config_get");

    await invoke("config_set", {
      config: {
        listenAddr: listenAddr.value,
        upstreamProxy: upstream.value || null,
        captureBody: cfg.captureBody ?? true,
        maxBodySize: cfg.maxBodySize,
        autoSystemProxy: autoSystemProxy.value,
        mitmHosts: cfg.mitmHosts || [],
      },
    });

    status.value = await invoke("proxy_start", {
      listenAddr: listenAddr.value,
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
