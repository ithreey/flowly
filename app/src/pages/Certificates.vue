<template>
  <div class="cert-page">
    <section class="cert-panel">
      <div class="panel-head">
        <div>
          <div class="panel-title">CA 证书</div>
          <div class="panel-subtitle">
            生成并安装用于 HTTPS 解密的本地根证书。
          </div>
        </div>
      </div>

      <div class="status-grid">
        <div class="status-card wide">
          <div class="status-label">证书文件</div>
          <div class="status-value">{{ status.path || "未生成" }}</div>
        </div>
        <div class="status-card">
          <div class="status-label">主题</div>
          <div class="status-value">{{ status.subject || "-" }}</div>
        </div>
        <div class="status-card">
          <div class="status-label">有效期至</div>
          <div class="status-value">{{ status.notAfter || "-" }}</div>
        </div>
      </div>

      <div class="actions">
        <el-button type="primary" :loading="busy" @click="installTrust">
          安装到系统信任区
        </el-button>
        <el-button :loading="busy" @click="copyPem">复制 PEM</el-button>
        <el-button :loading="busy" @click="generate(false)">重新生成</el-button>
        <el-button type="warning" plain :loading="busy" @click="generate(true)">
          强制重新生成
        </el-button>
      </div>

      <el-alert
        v-if="error"
        :title="error"
        type="error"
        :closable="false"
        show-icon
      />
      <el-alert
        v-if="info"
        :title="info"
        type="success"
        :closable="false"
        show-icon
      />
    </section>

    <section class="cert-panel guide-panel">
      <div class="panel-head">
        <div class="panel-title">使用说明</div>
      </div>
      <ol class="hint">
        <li>
          点击「安装到系统信任区」，让浏览器信任 Flowly 签发的 HTTPS 证书。
        </li>
        <li>「重新生成」会替换当前证书（已安装的信任将失效，需重新安装）。</li>
        <li>
          其他设备可从 <code>http://&lt;代理地址&gt;/mitm/cert</code> 下载证书。
        </li>
      </ol>
    </section>
  </div>
</template>

<script setup>
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { ElMessage } from "element-plus";

const status = ref({
  exists: false,
  path: "",
  subject: "",
  notAfter: "",
  certPem: "",
});
const busy = ref(false);
const error = ref("");
const info = ref("");

async function refresh() {
  try {
    status.value = await invoke("cert_status");
  } catch (e) {
    error.value = String(e);
  }
}

async function installTrust() {
  error.value = "";
  info.value = "";
  busy.value = true;
  try {
    await invoke("cert_install_trust");
    info.value = "证书已安装到系统信任区";
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = false;
  }
}

async function copyPem() {
  try {
    const pem = await invoke("cert_get_pem");
    await navigator.clipboard.writeText(pem);
    ElMessage.success("PEM 已复制");
  } catch (e) {
    error.value = String(e);
  }
}

async function generate(force) {
  error.value = "";
  info.value = "";
  busy.value = true;
  try {
    await invoke("cert_generate", { force });
    info.value = "证书已生成";
    await refresh();
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = false;
  }
}

onMounted(refresh);
</script>

<style scoped>
.cert-page {
  display: grid;
  max-width: 980px;
  gap: 16px;
}

.cert-panel {
  margin-bottom: 16px;
  border: 1px solid var(--gm-line);
  border-radius: 10px;
  background: rgba(15, 27, 45, 0.78);
  box-shadow: var(--gm-shadow);
}

.guide-panel {
  box-shadow: none;
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

.status-grid {
  display: grid;
  grid-template-columns: 1.4fr 1fr 1fr;
  gap: 10px;
  padding: 16px;
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

.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  padding: 0 16px 16px;
}

.hint {
  margin: 0;
  color: var(--gm-muted);
  font-size: 13px;
  line-height: 1.8;
  padding: 16px 16px 16px 36px;
}

.hint code {
  background: var(--gm-code);
  color: #cde6ff;
  padding: 2px 6px;
  border-radius: 4px;
}

@media (max-width: 900px) {
  .status-grid {
    grid-template-columns: 1fr;
  }
}
</style>
