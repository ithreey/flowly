<template>
  <el-drawer
    class="detail-drawer"
    :model-value="modelValue"
    size="62%"
    @update:model-value="$emit('update:modelValue', $event)"
  >
    <template #header>
      <div class="drawer-title">
        <span
          v-if="detail"
          class="method-tag"
          :class="methodClass(detail.summary.method)"
        >
          {{ detail.summary.method }}
        </span>
        <span class="drawer-url">{{ titleUrl }}</span>
      </div>
    </template>
    <div v-if="loading" class="placeholder">加载中...</div>
    <div v-else-if="error" class="placeholder">{{ error }}</div>
    <template v-else-if="detail">
      <div class="summary-grid">
        <div class="summary-card">
          <div class="summary-label">Status</div>
          <div
            class="summary-value"
            :class="statusClass(detail.summary)"
          >
            {{ statusText(detail.summary) }}
          </div>
        </div>
        <div class="summary-card">
          <div class="summary-label">Duration</div>
          <div class="summary-value">
            {{ formatDuration(detail.summary) }}
          </div>
        </div>
        <div class="summary-card">
          <div class="summary-label">Request</div>
          <div class="summary-value">
            {{ formatSize(detail.summary.reqSize) }}
          </div>
        </div>
        <div class="summary-card">
          <div class="summary-label">Response</div>
          <div class="summary-value">
            {{ formatSize(detail.summary.resSize) }}
          </div>
        </div>
      </div>
      <div class="detail-actions">
        <el-dropdown trigger="click" @command="handleCopyCommand">
          <el-button size="small" type="primary" plain>
            复制
            <el-icon class="el-icon--right"><ArrowDown /></el-icon>
          </el-button>
          <template #dropdown>
            <el-dropdown-menu>
              <el-dropdown-item command="url">复制 URL</el-dropdown-item>
              <el-dropdown-item command="curl"
                >复制 cURL(bash)</el-dropdown-item
              >
            </el-dropdown-menu>
          </template>
        </el-dropdown>
      </div>
      <el-tabs>
        <el-tab-pane label="请求">
          <h4>请求行</h4>
          <pre class="mono">{{ requestLine }}</pre>
          <h4>Headers</h4>
          <div class="copy-panel">
            <el-button
              class="copy-btn"
              size="small"
              text
              @click="copyText(formatHeaders(detail.reqHeaders))"
            >
              复制
            </el-button>
            <pre class="mono">{{ formatHeaders(detail.reqHeaders) }}</pre>
          </div>
          <h4>
            Body
            <el-radio-group
              v-model="reqBodyMode"
              size="small"
              class="body-mode"
            >
              <el-radio-button value="raw">原文</el-radio-button>
              <el-radio-button value="json">JSON</el-radio-button>
              <el-radio-button value="form">Form</el-radio-button>
            </el-radio-group>
          </h4>
          <div class="copy-panel">
            <el-button
              class="copy-btn"
              size="small"
              text
              @click="copyText(currentReqBody())"
            >
              复制
            </el-button>
            <pre
              v-if="reqBodyMode === 'raw'"
              class="body"
              :class="{ dim: !detail.reqBody }"
              >{{ detail.reqBody || "(无 body 或未捕获)" }}
            </pre>
            <template v-else-if="reqBodyMode === 'json'">
              <pre v-if="formatJson(detail.reqBody)" class="body">{{
                formatJson(detail.reqBody)
              }}</pre>
              <pre v-else class="body dim">{{
                detail.reqBody
                  ? "(不是有效的 JSON，请切回原文查看)"
                  : "(无 body 或未捕获)"
              }}</pre>
            </template>
            <template v-else>
              <pre
                v-if="formatForm(detail.reqHeaders, detail.reqBody)"
                class="body"
                >{{ formatForm(detail.reqHeaders, detail.reqBody) }}</pre>
              <pre v-else class="body dim">{{
                detail.reqBody
                  ? "(不是 application/x-www-form-urlencoded，请切回原文查看)"
                  : "(无 body 或未捕获)"
              }}</pre>
            </template>
          </div>
        </el-tab-pane>
        <el-tab-pane label="响应">
          <h4>状态行</h4>
          <pre class="mono">{{ responseLine }}</pre>
          <h4>Headers</h4>
          <div class="copy-panel">
            <el-button
              class="copy-btn"
              size="small"
              text
              @click="copyText(formatHeaders(detail.resHeaders))"
            >
              复制
            </el-button>
            <pre class="mono">{{ formatHeaders(detail.resHeaders) }}</pre>
          </div>
          <h4>
            Body
            <el-radio-group
              v-model="resBodyMode"
              size="small"
              class="body-mode"
            >
              <el-radio-button value="raw">原文</el-radio-button>
              <el-radio-button value="json">JSON</el-radio-button>
            </el-radio-group>
          </h4>
          <div class="copy-panel">
            <el-button
              class="copy-btn"
              size="small"
              text
              @click="copyText(currentResBody())"
            >
              复制
            </el-button>
            <pre
              v-if="resBodyMode === 'raw'"
              class="body"
              :class="{ dim: !detail.resBody }"
              >{{ detail.resBody || "(无 body 或未捕获)" }}
            </pre>
            <template v-else>
              <pre v-if="formatJson(detail.resBody)" class="body">{{
                formatJson(detail.resBody)
              }}</pre>
              <pre v-else class="body dim">{{
                detail.resBody
                  ? "(不是有效的 JSON，请切回原文查看)"
                  : "(无 body 或未捕获)"
              }}</pre>
            </template>
          </div>
        </el-tab-pane>
        <el-tab-pane label="时间线">
          <div class="timeline-panel">
            <div class="timeline-row">
              <span>开始时间</span>
              <strong>{{ formatDateTime(detail.summary.startedAt) }}</strong>
            </div>
            <div class="timeline-row">
              <span>总耗时</span>
              <strong>{{ formatDuration(detail.summary) }}</strong>
            </div>
            <div class="timeline-bar">
              <div class="timeline-fill"></div>
            </div>
            <div class="timeline-note">
              当前捕获数据提供会话级耗时；后续可扩展
              DNS、连接、发送、等待、接收阶段。
            </div>
          </div>
        </el-tab-pane>
      </el-tabs>
    </template>
    <div v-else class="placeholder">
      该会话详情已过期或已被缓存淘汰，请重新抓取。
    </div>
  </el-drawer>
</template>

<script setup>
import { ref, computed, watch } from "vue";
import { ElMessage } from "element-plus";
import { ArrowDown } from "@element-plus/icons-vue";
import { useTrafficStore } from "../stores/traffic";
import { isFormUrlEncoded, parseFormParams } from "../utils/har";

const props = defineProps({
  modelValue: { type: Boolean, default: false },
  id: { type: Number, default: null },
});
defineEmits(["update:modelValue"]);

const traffic = useTrafficStore();
const detail = ref(null);
const loading = ref(false);
const error = ref("");

// body 展示模式：默认原文。
const reqBodyMode = ref("raw");
const resBodyMode = ref("raw");

/** 文本若为合法 JSON 则格式化返回，否则返回 null。 */
function formatJson(text) {
  if (!text) return null;
  try {
    return JSON.stringify(JSON.parse(text), null, 2);
  } catch {
    return null;
  }
}

function formatForm(headers, text) {
  if (!text || !isFormUrlEncoded(headers || [])) return null;
  const params = parseFormParams(text);
  if (params.length === 0) return null;
  return params.map(({ name, value }) => `${name}: ${value}`).join("\n");
}

function formatSize(size) {
  if (!size) return "0B";
  if (size < 1024) return `${size}B`;
  if (size < 1024 * 1024) return `${(size / 1024).toFixed(1)}K`;
  return `${(size / 1024 / 1024).toFixed(1)}M`;
}

function formatDateTime(millis) {
  if (!millis) return "-";
  const d = new Date(Number(millis));
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(
    d.getDate(),
  ).padStart(2, "0")} ${String(d.getHours()).padStart(2, "0")}:${String(
    d.getMinutes(),
  ).padStart(2, "0")}:${String(d.getSeconds()).padStart(2, "0")}`;
}

function methodClass(method) {
  const value = String(method || "").toLowerCase();
  if (["get", "post", "delete"].includes(value)) return `method-${value}`;
  if (["put", "patch"].includes(value)) return "method-patch";
  return "method-other";
}

function statusClass(summary) {
  if (summary.phase === "pending") return "status-pending";
  if (summary.phase === "failed") return "status-err";
  const status = summary.status;
  if (status == null) return "status-none";
  if (status >= 200 && status < 400) return "status-ok";
  if (status >= 400 && status < 500) return "status-warn";
  return "status-err";
}

function statusText(summary) {
  if (summary.phase === "pending") return "等待";
  if (summary.phase === "failed") return "失败";
  return summary.status ?? "-";
}

function formatDuration(summary) {
  if (summary.phase === "pending") return "-";
  return summary.durationMs == null ? "-" : `${summary.durationMs}ms`;
}

/** 当前请求 body 的展示文本（JSON 模式下复制格式化后的内容）。 */
function currentReqBody() {
  if (!detail.value) return "";
  const text = detail.value.reqBody || "";
  if (reqBodyMode.value === "json" && formatJson(text)) return formatJson(text);
  if (
    reqBodyMode.value === "form" &&
    formatForm(detail.value.reqHeaders, text)
  ) {
    return formatForm(detail.value.reqHeaders, text);
  }
  return text;
}

/** 当前响应 body 的展示文本。 */
function currentResBody() {
  if (!detail.value) return "";
  const text = detail.value.resBody || "";
  if (resBodyMode.value === "json" && formatJson(text)) return formatJson(text);
  return text;
}

/** 复制文本到剪贴板。 */
async function copyText(text) {
  try {
    await navigator.clipboard.writeText(text);
    ElMessage.success("已复制");
  } catch (e) {
    ElMessage.error("复制失败: " + String(e));
  }
}

function handleCopyCommand(command) {
  if (!detail.value) return;
  if (command === "url") {
    copyText(detail.value.summary.url || "");
    return;
  }
  if (command === "curl") {
    copyText(buildCurlCommand());
  }
}

function shellQuote(value) {
  return `'${String(value).replace(/'/g, `'\\''`)}'`;
}

function buildCurlCommand() {
  if (!detail.value) return "";
  const txn = detail.value;
  const parts = [
    "curl",
    "-X",
    shellQuote(txn.summary.method || "GET"),
    shellQuote(txn.summary.url || ""),
  ];

  for (const [name, value] of txn.reqHeaders || []) {
    if (!name) continue;
    parts.push("-H", shellQuote(`${name}: ${value}`));
  }

  if (txn.reqBody) {
    parts.push("--data-raw", shellQuote(txn.reqBody));
  }

  return parts
    .map((part, index) => (index === 0 ? part : `  ${part}`))
    .join(" \\\n");
}

const title = computed(() =>
  detail.value
    ? `${detail.value.summary.method} ${detail.value.summary.url}`
    : "详情",
);

const titleUrl = computed(() =>
  detail.value ? detail.value.summary.url : "详情",
);

const requestLine = computed(() => {
  if (!detail.value) return "";
  const s = detail.value.summary;
  return `${s.method} ${s.url}`;
});

const responseLine = computed(() => {
  if (!detail.value) return "";
  const s = detail.value.summary;
  return `HTTP/1.1 ${s.status ?? "-"} ${detail.value.resHeaders
    .filter(([k]) => k.toLowerCase() === "content-type")
    .map(([, v]) => v)
    .join("; ")}`;
});

function formatHeaders(headers) {
  if (!headers || headers.length === 0) return "(无)";
  return headers.map(([k, v]) => `${k}: ${v}`).join("\n");
}

watch(
  () => [props.modelValue, props.id],
  async ([visible, id]) => {
    if (visible && id != null) {
      loading.value = true;
      error.value = "";
      detail.value = null;
      try {
        const result = await traffic.getDetail(id);
        detail.value = result;
      } catch (e) {
        error.value = String(e);
        detail.value = null;
      } finally {
        loading.value = false;
      }
    } else {
      detail.value = null;
    }
  },
);
</script>

<style scoped>
.placeholder {
  color: var(--gm-muted);
  padding: 24px;
  text-align: center;
}

.drawer-title {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  color: var(--gm-text);
  font-size: 18px;
  font-weight: 500;
  line-height: 1.35;
}

:global(.detail-drawer .el-drawer__header) {
  margin-bottom: 0;
  padding: 16px;
  border-bottom: 1px solid var(--gm-line);
}

:global(.detail-drawer .el-drawer__body) {
  padding: 0 16px 16px;
  background: var(--gm-panel);
}

.method-tag {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  min-width: 52px;
  height: 24px;
  margin-top: 1px;
  padding: 0 8px;
  border-radius: 7px;
  font-family: "Cascadia Mono", "JetBrains Mono", Consolas, monospace;
  font-size: 12px;
  font-weight: 700;
}

.method-get {
  background: rgba(56, 189, 248, 0.13);
  color: #7dd3fc;
}

.method-post {
  background: rgba(167, 139, 250, 0.14);
  color: #c4b5fd;
}

.method-patch {
  background: rgba(245, 158, 11, 0.13);
  color: #fcd34d;
}

.method-delete {
  background: rgba(248, 113, 113, 0.13);
  color: #fca5a5;
}

.method-other {
  background: rgba(148, 163, 184, 0.12);
  color: var(--gm-muted);
}

.drawer-url {
  min-width: 0;
  color: #dbeafe;
  font-family: "Cascadia Mono", "JetBrains Mono", Consolas, monospace;
  font-size: 13px;
  white-space: normal;
  word-break: break-all;
}

.summary-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 8px;
  padding: 12px 0;
}

.summary-card {
  padding: 10px;
  border: 1px solid var(--gm-line);
  border-radius: 8px;
  background: rgba(8, 17, 31, 0.42);
}

.summary-label {
  color: var(--gm-subtle);
  font-size: 11px;
  text-transform: uppercase;
}

.summary-value {
  margin-top: 5px;
  color: var(--gm-text);
  font-family: "Cascadia Mono", "JetBrains Mono", Consolas, monospace;
  font-weight: 700;
}

.summary-value.status-ok {
  color: #86efac;
}

.summary-value.status-pending {
  color: #7dd3fc;
}

.summary-value.status-warn {
  color: #fcd34d;
}

.summary-value.status-err {
  color: #fca5a5;
}

h4 {
  margin: 16px 0 6px;
  color: var(--gm-muted);
  font-size: 13px;
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.body-mode {
  margin-left: 8px;
  margin-right: 2px;
  height: 32px;
}

.body-mode :deep(.el-radio-button__inner) {
  height: 32px;
  min-height: 32px;
  line-height: 30px;
  padding-top: 0;
  padding-bottom: 0;
}

.detail-actions {
  display: flex;
  justify-content: flex-start;
  margin-bottom: 8px;
}

.copy-panel {
  position: relative;
}

.copy-btn {
  position: absolute;
  top: 4px;
  right: 4px;
  z-index: 1;
  background: rgba(8, 17, 31, 0.92);
}

.mono {
  font-family: Consolas, "Courier New", monospace;
  font-size: 12px;
  background: var(--gm-code);
  border: 1px solid rgba(56, 189, 248, 0.18);
  color: #cde6ff;
  padding: 8px 10px;
  border-radius: 6px;
  margin: 0;
  white-space: pre-wrap;
  word-break: break-all;
  max-height: 300px;
  overflow: auto;
}

.body {
  font-family: Consolas, "Courier New", monospace;
  font-size: 12px;
  background: var(--gm-code);
  border: 1px solid rgba(56, 189, 248, 0.18);
  color: #cde6ff;
  padding: 8px 10px;
  border-radius: 6px;
  margin: 0;
  white-space: pre-wrap;
  word-break: break-all;
  max-height: 400px;
  overflow: auto;
}

.dim {
  color: var(--gm-subtle);
}

.timeline-panel {
  padding: 12px;
  border: 1px solid var(--gm-line);
  border-radius: 8px;
  background: rgba(8, 17, 31, 0.42);
}

.timeline-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 8px 0;
  color: var(--gm-muted);
}

.timeline-row strong {
  color: var(--gm-text);
  font-family: "Cascadia Mono", "JetBrains Mono", Consolas, monospace;
}

.timeline-bar {
  height: 8px;
  margin: 12px 0;
  overflow: hidden;
  border-radius: 999px;
  background: rgba(148, 163, 184, 0.12);
}

.timeline-fill {
  width: 72%;
  height: 100%;
  border-radius: inherit;
  background: linear-gradient(90deg, var(--gm-cyan), var(--gm-green));
}

.timeline-note {
  color: var(--gm-subtle);
  font-size: 12px;
  line-height: 1.7;
}

@media (max-width: 900px) {
  .summary-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}
</style>
