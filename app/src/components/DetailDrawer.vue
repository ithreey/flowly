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
        <span class="drawer-url" :class="{ expanded: urlExpanded }">{{
          titleUrl
        }}</span>
        <el-button
          v-if="showUrlToggle"
          class="url-toggle"
          text
          circle
          size="small"
          @click.stop="urlExpanded = !urlExpanded"
        >
          <el-icon>
            <ArrowUp v-if="urlExpanded" />
            <ArrowDown v-else />
          </el-icon>
        </el-button>
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
      <el-tabs v-model="activeTab">
        <el-tab-pane label="请求" name="request">
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
              :disabled="detail.reqBodyAvailable && !reqBodyLoaded"
              @click="copyText(currentReqBody())"
            >
              复制
            </el-button>
            <pre v-if="reqBodyLoading" class="body dim">加载 Body 中...</pre>
            <pre v-else-if="reqBodyError" class="body dim">{{
              reqBodyError
            }}</pre>
            <div
              v-else-if="detail.reqBodyAvailable && !reqBodyLoaded"
              class="body-lazy"
            >
              <el-button
                size="small"
                type="primary"
                plain
                @click="loadBody('request')"
              >
                加载请求 Body ({{ formatSize(detail.summary.reqSize) }})
              </el-button>
            </div>
            <pre
              v-else-if="reqBodyMode === 'raw'"
              class="body"
              :class="{ dim: !reqBody }"
              >{{ reqBody || "(无 body 或未捕获)" }}
            </pre>
            <template v-else-if="reqBodyMode === 'json'">
              <pre v-if="reqBodyJson" class="body">{{ reqBodyJson }}</pre>
              <pre v-else class="body dim">{{
                reqBody
                  ? "(不是有效的 JSON，请切回原文查看)"
                  : "(无 body 或未捕获)"
              }}</pre>
            </template>
            <template v-else>
              <pre v-if="reqBodyForm" class="body">{{ reqBodyForm }}</pre>
              <pre v-else class="body dim">{{
                reqBody
                  ? "(不是 application/x-www-form-urlencoded，请切回原文查看)"
                  : "(无 body 或未捕获)"
              }}</pre>
            </template>
          </div>
        </el-tab-pane>
        <el-tab-pane label="响应" name="response">
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
              :disabled="detail.resBodyAvailable && !resBodyLoaded"
              @click="copyText(currentResBody())"
            >
              复制
            </el-button>
            <pre v-if="resBodyLoading" class="body dim">加载 Body 中...</pre>
            <pre v-else-if="resBodyError" class="body dim">{{
              resBodyError
            }}</pre>
            <div
              v-else-if="detail.resBodyAvailable && !resBodyLoaded"
              class="body-lazy"
            >
              <el-button
                size="small"
                type="primary"
                plain
                @click="loadBody('response')"
              >
                加载响应 Body ({{ formatSize(detail.summary.resSize) }})
              </el-button>
            </div>
            <pre
              v-else-if="resBodyMode === 'raw'"
              class="body"
              :class="{ dim: !resBody }"
              >{{ resBody || "(无 body 或未捕获)" }}
            </pre>
            <template v-else>
              <pre v-if="resBodyJson" class="body">{{ resBodyJson }}</pre>
              <pre v-else class="body dim">{{
                resBody
                  ? "(不是有效的 JSON，请切回原文查看)"
                  : "(无 body 或未捕获)"
              }}</pre>
            </template>
          </div>
        </el-tab-pane>
        <el-tab-pane label="时间线" name="timeline">
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
import { ArrowDown, ArrowUp } from "@element-plus/icons-vue";
import { useTrafficStore } from "../stores/traffic";
import {
  shouldCollapseUrlAfterSelectionChange,
  shouldShowUrlToggle,
} from "../utils/detail-url-collapse";
import { shouldAutoLoadBody } from "../utils/detail-body-loading";
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
const urlExpanded = ref(false);
const activeTab = ref("request");
const reqBody = ref("");
const resBody = ref("");
const reqBodyLoaded = ref(false);
const resBodyLoaded = ref(false);
const reqBodyLoading = ref(false);
const resBodyLoading = ref(false);
const reqBodyError = ref("");
const resBodyError = ref("");
let detailLoadVersion = 0;

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
  const text = reqBody.value || "";
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
  const text = resBody.value || "";
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

const showUrlToggle = computed(() =>
  shouldShowUrlToggle(detail.value?.summary?.url),
);

const reqBodyJson = computed(() => formatJson(reqBody.value));
const resBodyJson = computed(() => formatJson(resBody.value));
const reqBodyForm = computed(() =>
  detail.value ? formatForm(detail.value.reqHeaders, reqBody.value) : null,
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

function resetBodies() {
  reqBody.value = "";
  resBody.value = "";
  reqBodyLoaded.value = false;
  resBodyLoaded.value = false;
  reqBodyLoading.value = false;
  resBodyLoading.value = false;
  reqBodyError.value = "";
  resBodyError.value = "";
}

function bodyAvailable(kind) {
  if (!detail.value) return false;
  return kind === "request"
    ? detail.value.reqBodyAvailable
    : detail.value.resBodyAvailable;
}

function bodySize(kind) {
  if (!detail.value) return 0;
  return kind === "request"
    ? detail.value.summary.reqSize
    : detail.value.summary.resSize;
}

function bodyLoaded(kind) {
  return kind === "request" ? reqBodyLoaded.value : resBodyLoaded.value;
}

function canAutoLoadBody(kind) {
  return shouldAutoLoadBody({
    available: bodyAvailable(kind),
    size: bodySize(kind),
  });
}

function loadAutoBodyForActiveTab() {
  const kind = activeTab.value === "response" ? "response" : "request";
  if (canAutoLoadBody(kind) && !bodyLoaded(kind)) {
    loadBody(kind);
  }
}

async function loadBody(kind) {
  if (!detail.value) return;
  const id = detail.value.summary.id;
  const isRequest = kind === "request";
  const loaded = isRequest ? reqBodyLoaded : resBodyLoaded;
  const loadingState = isRequest ? reqBodyLoading : resBodyLoading;
  const errorState = isRequest ? reqBodyError : resBodyError;
  const bodyState = isRequest ? reqBody : resBody;

  if (loaded.value || loadingState.value) return;
  loadingState.value = true;
  errorState.value = "";
  try {
    const body = await traffic.getBody(id, kind);
    if (detail.value?.summary?.id !== id) return;
    bodyState.value = body || "";
    loaded.value = true;
  } catch (e) {
    if (detail.value?.summary?.id !== id) return;
    errorState.value = `加载 Body 失败: ${String(e)}`;
  } finally {
    if (detail.value?.summary?.id === id) {
      loadingState.value = false;
    }
  }
}

watch(
  () => [props.modelValue, props.id],
  async ([visible, id], [, previousId] = []) => {
    if (
      shouldCollapseUrlAfterSelectionChange({
        visible,
        currentId: id,
        previousId,
      })
    ) {
      urlExpanded.value = false;
    }
    if (visible && id != null) {
      const loadVersion = ++detailLoadVersion;
      loading.value = true;
      error.value = "";
      detail.value = null;
      activeTab.value = "request";
      resetBodies();
      try {
        const result = await traffic.getDetailMeta(id);
        if (loadVersion !== detailLoadVersion || !props.modelValue || props.id !== id) return;
        detail.value = result;
        loadAutoBodyForActiveTab();
      } catch (e) {
        if (loadVersion !== detailLoadVersion) return;
        error.value = String(e);
        detail.value = null;
      } finally {
        if (loadVersion === detailLoadVersion) {
          loading.value = false;
        }
      }
    } else {
      detailLoadVersion += 1;
      detail.value = null;
      urlExpanded.value = false;
      activeTab.value = "request";
      resetBodies();
    }
  },
);

watch(activeTab, () => {
  loadAutoBodyForActiveTab();
});
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
  flex: 1;
  color: #dbeafe;
  font-family: "Cascadia Mono", "JetBrains Mono", Consolas, monospace;
  font-size: 13px;
  display: -webkit-box;
  overflow: hidden;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
  word-break: break-all;
}

.drawer-url.expanded {
  display: block;
  overflow: visible;
  -webkit-line-clamp: unset;
}

.url-toggle {
  flex: 0 0 auto;
  margin-top: -2px;
  color: var(--gm-muted);
}

.url-toggle:hover,
.url-toggle:focus {
  color: var(--gm-cyan);
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

.body-lazy {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 96px;
  border: 1px dashed rgba(56, 189, 248, 0.24);
  border-radius: 6px;
  background: rgba(7, 16, 29, 0.58);
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
