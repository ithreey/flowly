<template>
  <div class="response-viewer">
    <!-- 状态栏 -->
    <div class="response-status-bar">
      <span class="response-label">Response</span>
      <template v-if="store.response">
        <el-tag :type="statusTagType" size="small" effect="dark" class="status-tag">
          {{ store.response.status }} {{ store.response.statusText }}
        </el-tag>
        <span class="stat">{{ store.response.durationMs }}ms</span>
        <span class="stat">{{ formatSize(store.response.size) }}</span>
      </template>
      <span v-else-if="store.error" class="error-text">{{ store.error }}</span>
      <span v-else-if="store.sending" class="stat">
        <el-icon class="spinning"><Loading /></el-icon> 请求中...
      </span>
      <span v-else class="empty-hint">点击 Send 发送请求</span>
    </div>

    <!-- Tab 面板 -->
    <el-tabs v-if="store.response" v-model="activeTab" class="response-tabs">
      <el-tab-pane label="Body" name="body">
        <div class="body-sub-tabs">
          <el-checkbox v-model="prettyEnabled" size="small">Pretty</el-checkbox>
        </div>
        <div v-if="prettyEnabled" class="body-content">
          <codemirror
            :model-value="prettyBody"
            :style="{ height: '100%', fontSize: '13px' }"
            :extensions="prettyExtensions"
            :readonly="true"
          />
        </div>
        <pre v-else class="raw-body">{{ bodyText }}</pre>
      </el-tab-pane>
      <el-tab-pane label="Headers" name="headers">
        <table class="headers-table">
          <tbody>
            <tr v-for="(h, i) in store.response.headers" :key="i">
              <td class="header-name">{{ h[0] }}</td>
              <td class="header-value">{{ h[1] }}</td>
            </tr>
          </tbody>
        </table>
      </el-tab-pane>
      <el-tab-pane label="Preview" name="preview">
        <div v-if="previewType === 'html'" class="preview-frame">
          <iframe :srcdoc="bodyText" sandbox="" class="preview-iframe" />
        </div>
        <div v-else-if="previewType === 'image'" class="preview-image">
          <img :src="imageDataUrl" alt="preview" />
        </div>
        <div v-else class="body-empty">此内容类型不支持预览</div>
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup>
import { ref, computed, watch, onBeforeUnmount } from "vue";
import { Loading } from "@element-plus/icons-vue";
import { Codemirror } from "vue-codemirror";
import { json } from "@codemirror/lang-json";
import { xml } from "@codemirror/lang-xml";
import { useSenderStore } from "../stores/sender";

const store = useSenderStore();
const activeTab = ref("body");
const prettyEnabled = ref(true);
const imageDataUrl = ref("");

const statusTagType = computed(() => {
  const s = store.response?.status;
  if (!s) return "info";
  if (s < 300) return "success";
  if (s < 400) return "";
  if (s < 500) return "warning";
  return "danger";
});

const contentType = computed(() => {
  const h = store.response?.headers?.find(
    ([k]) => k.toLowerCase() === "content-type"
  );
  return h ? h[1].toLowerCase() : "";
});

const bodyBytes = computed(() => store.response?.bodyBytes || []);
const responseUrl = computed(() => store.response?.url || "");

const bodyText = computed(() => {
  const bytes = bodyBytes.value;
  if (!bytes.length) return "";
  try {
    return new TextDecoder("utf-8", { fatal: false }).decode(
      new Uint8Array(bytes)
    );
  } catch {
    return "[二进制数据，无法显示为文本]";
  }
});

const prettyExtensions = computed(() => {
  const ct = contentType.value;
  if (ct.includes("json")) return [json()];
  if (ct.includes("xml") || ct.includes("html")) return [xml()];
  return [];
});

const prettyBody = computed(() => {
  const text = bodyText.value;
  if (contentType.value.includes("json")) {
    try {
      return JSON.stringify(JSON.parse(text), null, 2);
    } catch {
      return text;
    }
  }
  return text;
});

const previewType = computed(() => {
  const ct = contentType.value;
  if (ct.includes("html")) return "html";
  if (isImageResponse.value) return "image";
  return "none";
});

const isImageResponse = computed(() => {
  if (contentType.value.startsWith("image/")) return true;
  return /\.(png|jpe?g|gif|webp|bmp|svg)(\?.*)?$/i.test(responseUrl.value);
});

const imageContentType = computed(() => {
  if (contentType.value.startsWith("image/")) return contentType.value;
  const url = responseUrl.value.toLowerCase();
  if (url.includes(".svg")) return "image/svg+xml";
  if (url.includes(".webp")) return "image/webp";
  if (url.includes(".gif")) return "image/gif";
  if (url.includes(".bmp")) return "image/bmp";
  if (url.includes(".jpg") || url.includes(".jpeg")) return "image/jpeg";
  return "image/png";
});

watch(
  [bodyBytes, contentType, responseUrl],
  ([bytes]) => {
    if (imageDataUrl.value) {
      URL.revokeObjectURL(imageDataUrl.value);
      imageDataUrl.value = "";
    }
    if (!bytes.length || !isImageResponse.value) return;
    const blob = new Blob([new Uint8Array(bytes)], {
      type: imageContentType.value,
    });
    imageDataUrl.value = URL.createObjectURL(blob);
  },
  { immediate: true }
);

onBeforeUnmount(() => {
  if (imageDataUrl.value) URL.revokeObjectURL(imageDataUrl.value);
});

function formatSize(bytes) {
  if (bytes == null) return "";
  if (bytes < 1024) return bytes + " B";
  return (bytes / 1024).toFixed(1) + " KB";
}
</script>

<style scoped>
.response-viewer {
  display: flex;
  flex-direction: column;
}
.response-status-bar {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 0;
  border-bottom: 1px solid var(--gm-line);
  margin-bottom: 8px;
  flex-shrink: 0;
}
.response-label {
  font-size: 13px;
  font-weight: 600;
  color: var(--gm-text);
}
.stat {
  font-size: 12px;
  color: var(--gm-subtle);
}
.error-text {
  font-size: 12px;
  color: #f87171;
}
.empty-hint {
  font-size: 12px;
  color: var(--gm-subtle);
}
.spinning {
  animation: spin 1s linear infinite;
}
@keyframes spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}
.response-tabs :deep(.el-tabs__header) {
  margin-bottom: 8px;
}
.body-sub-tabs {
  margin-bottom: 8px;
  flex-shrink: 0;
}
.body-content {
  border: 1px solid var(--gm-line);
  border-radius: 4px;
  overflow: hidden;
  height: calc(100% - 40px);
}
.raw-body {
  font-family: "JetBrains Mono", "Fira Code", monospace;
  font-size: 13px;
  color: var(--gm-text);
  white-space: pre-wrap;
  word-break: break-all;
  padding: 12px;
  margin: 0;
  max-height: 400px;
  overflow-y: auto;
}
.headers-table {
  width: 100%;
  font-size: 12px;
  border-collapse: collapse;
}
.headers-table td {
  padding: 4px 8px;
  border-bottom: 1px solid var(--gm-line);
}
.header-name {
  color: #38bdf8;
  font-weight: 600;
  width: 200px;
  white-space: nowrap;
}
.header-value {
  color: var(--gm-text);
  word-break: break-all;
}
.body-empty {
  color: var(--gm-subtle);
  font-size: 12px;
  padding: 24px;
  text-align: center;
}
.preview-frame {
  border: 1px solid var(--gm-line);
  border-radius: 4px;
  overflow: hidden;
  height: 300px;
}
.preview-iframe {
  width: 100%;
  height: 100%;
  border: none;
  background: #fff;
}
.preview-image img {
  max-width: 100%;
  max-height: 300px;
}
</style>
