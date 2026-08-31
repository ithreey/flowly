<template>
  <div class="monitor-page">
    <section class="monitor-panel">
      <div class="toolbar">
        <div class="filter-group">
          <el-select
            v-model="methodFilter"
            multiple
            clearable
            placeholder="请求方法"
            size="small"
            class="method-filter"
          >
            <el-option
              v-for="method in methodOptions"
              :key="method"
              :label="method"
              :value="method"
            />
          </el-select>
          <el-input
            v-model="filterText"
            placeholder="URL 过滤（输入 URL 包含词）"
            clearable
            class="filter-input"
          />
        </div>
        <div class="batch-actions">
          <span
            class="selection-chip"
            :class="{ active: selectedRows.length > 0 }"
          >
            已选 {{ selectedRows.length }}
          </span>
          <el-button
            type="success"
            :disabled="selectedRows.length === 0"
            :loading="exporting"
            @click="exportToHar"
          >
            {{
              exporting
                ? "导出中..."
                : `导出${selectedRows.length > 0 ? ` (${selectedRows.length})` : ""}`
            }}
          </el-button>
          <el-button type="danger" plain @click="clear">清空</el-button>
        </div>
      </div>

      <el-table
        ref="tableRef"
        :data="filteredList"
        row-key="id"
        size="small"
        height="100%"
        @row-click="openDetail"
        @selection-change="handleSelectionChange"
        @row-contextmenu="handleContextMenu"
        empty-text="暂无流量。启动代理并配置系统代理后，新的 HTTP/HTTPS 会话会显示在这里。"
      >
        <el-table-column type="selection" width="55" reserve-selection />
        <el-table-column label="方法" width="104">
          <template #default="{ row }">
            <span class="method-pill" :class="methodClass(row.method)">
              {{ row.method || "-" }}
            </span>
          </template>
        </el-table-column>
        <el-table-column label="状态" width="86">
          <template #default="{ row }">
            <span class="status-pill-cell" :class="statusClass(row)">
              <el-icon class="status-icon" :class="{ spinning: row.phase === 'pending' }">
                <component :is="statusIcon(row)" />
              </el-icon>
              <span>{{ statusText(row) }}</span>
            </span>
          </template>
        </el-table-column>
        <el-table-column label="URL" min-width="300">
          <template #default="{ row }">
            <div class="url-cell">
              <div class="url-host">{{ splitUrl(row.url).host }}</div>
              <div class="url-path">{{ splitUrl(row.url).path }}</div>
            </div>
          </template>
        </el-table-column>
        <el-table-column label="大小" width="138" align="right">
          <template #default="{ row }">
            {{ formatSize(row.reqSize) }} / {{ formatSize(row.resSize) }}
          </template>
        </el-table-column>
        <el-table-column label="耗时" width="92" align="right">
          <template #default="{ row }">{{ formatDuration(row) }}</template>
        </el-table-column>
        <el-table-column label="时间" width="90">
          <template #default="{ row }">{{
            formatTime(row.startedAt)
          }}</template>
        </el-table-column>
      </el-table>
    </section>

    <!-- 右键菜单 -->
    <div
      v-if="contextMenuVisible"
      ref="contextMenuRef"
      class="context-menu"
      :style="{ left: contextMenuX + 'px', top: contextMenuY + 'px' }"
    >
      <div class="context-menu-item" @click="addToSender">添加到发送器</div>
      <div class="context-menu-item" @click="replayRequest">重放请求</div>
      <div class="context-menu-item" @click="exportToHar">导出选中为 HAR</div>
      <div class="context-menu-item danger" @click="deleteSelected">
        删除选中会话
      </div>
    </div>

    <DetailDrawer v-model="drawerVisible" :id="selectedId" />
  </div>
</template>

<script setup>
import { ref, computed, watch, nextTick, onMounted, onBeforeUnmount } from "vue";
import { ElMessage } from "element-plus";
import { CircleCheck, CircleClose, Loading, WarningFilled } from "@element-plus/icons-vue";
import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import { writeTextFile } from "@tauri-apps/plugin-fs";
import { useRouter } from "vue-router";
import { useSenderStore } from "../stores/sender";
import { useTrafficStore } from "../stores/traffic";
import DetailDrawer from "../components/DetailDrawer.vue";
import { transactionToHarEntry, generateHarFile } from "../utils/har";

const router = useRouter();
const sender = useSenderStore();
const traffic = useTrafficStore();
const running = ref(false);
const busy = ref(false);
const drawerVisible = ref(false);
const selectedId = ref(null);
const tableRef = ref(null);
// 记录用户是否手动向上滚动过（偏离底部超过阈值则视为手动滚动）
const autoScrollEnabled = ref(true);
const SCROLL_THRESHOLD = 60;

// 导出相关状态
const selectedRows = ref([]);
const exporting = ref(false);
const contextMenuVisible = ref(false);
const contextMenuX = ref(0);
const contextMenuY = ref(0);
const contextMenuRow = ref(null);
const contextMenuRef = ref(null);
const CONTEXT_MENU_MARGIN = 8;

const methodOptions = [
  "GET",
  "POST",
  "PUT",
  "PATCH",
  "DELETE",
  "HEAD",
  "OPTIONS",
];
const methodFilter = ref(["POST", "GET"]);

// 前端过滤：请求方法可多选，URL 输入词时按包含匹配。
const filterText = ref("");
const filteredList = computed(() => {
  const kw = filterText.value.trim();
  const methods = methodFilter.value;
  return traffic.list.filter((t) => {
    const methodMatched = methods.length === 0 || methods.includes(t.method);
    const urlMatched = !kw || (t.url && t.url.includes(kw));
    return methodMatched && urlMatched;
  });
});

function statusClass(row) {
  if (row.phase === "pending") return "status-pending";
  if (row.phase === "failed") return "status-err";
  const status = row.status;
  if (status == null) return "status-none";
  if (status >= 200 && status < 400) return "status-ok";
  if (status >= 400 && status < 500) return "status-warn";
  return "status-err";
}

function statusIcon(row) {
  if (row.phase === "pending") return Loading;
  if (row.phase === "failed") return CircleClose;
  const status = row.status;
  if (status == null) return Loading;
  if (status >= 200 && status < 400) return CircleCheck;
  if (status >= 400 && status < 500) return WarningFilled;
  return CircleClose;
}

function statusText(row) {
  if (row.phase === "pending") return "等待";
  if (row.phase === "failed") return "失败";
  return row.status ?? "-";
}

function methodClass(method) {
  const value = String(method || "").toLowerCase();
  if (["get", "post", "delete"].includes(value)) return `method-${value}`;
  if (["put", "patch"].includes(value)) return "method-patch";
  return "method-other";
}

function splitUrl(url) {
  if (!url) return { host: "-", path: "" };
  try {
    const parsed = new URL(url);
    return {
      host: parsed.host,
      path: `${parsed.pathname}${parsed.search}`,
    };
  } catch {
    const [host, ...rest] = String(url).split("/");
    return { host, path: rest.length ? `/${rest.join("/")}` : "" };
  }
}

function formatSize(size) {
  if (!size) return "0B";
  if (size < 1024) return `${size}B`;
  if (size < 1024 * 1024) return `${(size / 1024).toFixed(1)}K`;
  return `${(size / 1024 / 1024).toFixed(1)}M`;
}

function formatTime(millis) {
  if (!millis) return "";
  const d = new Date(Number(millis));
  return `${d.getHours().toString().padStart(2, "0")}:${d
    .getMinutes()
    .toString()
    .padStart(2, "0")}:${d.getSeconds().toString().padStart(2, "0")}`;
}

async function refreshProxyStatus() {
  try {
    const status = await invoke("proxy_status");
    running.value = status.running;
  } catch {
    running.value = false;
  }
}

/** 用「代理设置」页的配置启动代理（含自动系统代理）。 */
async function start() {
  busy.value = true;
  try {
    const cfg = await invoke("config_get");
    await invoke("proxy_start", {
      listenAddr: cfg.listenAddr,
      upstreamProxy: cfg.upstreamProxy || null,
    });
    running.value = true;
    ElMessage.success("代理已启动");
  } catch (e) {
    ElMessage.error(String(e));
  } finally {
    busy.value = false;
  }
}

async function stop() {
  busy.value = true;
  try {
    await invoke("proxy_stop");
    running.value = false;
    ElMessage.success("代理已停止");
  } catch (e) {
    ElMessage.error(String(e));
  } finally {
    busy.value = false;
  }
}

async function openDetail(row, column) {
  if (column?.type === "selection") return; // Don't open drawer for checkbox clicks
  selectedId.value = row.id;
  drawerVisible.value = true;
}

function handleSelectionChange(selection) {
  selectedRows.value = selection;
}

async function handleContextMenu(row, column, event) {
  event.preventDefault();
  contextMenuRow.value = row;
  contextMenuX.value = event.clientX;
  contextMenuY.value = event.clientY;
  contextMenuVisible.value = true;
  await nextTick();
  fitContextMenuInViewport();
}

function formatDuration(row) {
  if (row.phase === "pending") return "-";
  return row.durationMs == null ? "-" : `${row.durationMs}ms`;
}

// 点击其他地方关闭右键菜单
const handleClick = () => {
  contextMenuVisible.value = false;
  contextMenuRow.value = null;
};

async function replayRequest() {
  const row = contextMenuRow.value;
  if (!row) return;
  contextMenuVisible.value = false;
  contextMenuRow.value = null;
  try {
    await traffic.replay(row.id);
    ElMessage.success("已重放请求");
  } catch (e) {
    ElMessage.error(`重放失败：${e}`);
  }
}

async function addToSender() {
  const row = contextMenuRow.value;
  if (!row) return;
  contextMenuVisible.value = false;
  contextMenuRow.value = null;

  try {
    const detail = await traffic.getDetail(row.id);
    if (!detail) {
      ElMessage.warning("会话详情已过期，无法添加到发送器");
      return;
    }

    sender.loadFromTrafficDetail(detail);
    await router.push("/sender");
    ElMessage.success("已添加到发送器");
  } catch (e) {
    ElMessage.error(`添加到发送器失败：${e}`);
  }
}

function fitContextMenuInViewport() {
  const menu = contextMenuRef.value;
  if (!menu) return;

  const rect = menu.getBoundingClientRect();
  const maxX = window.innerWidth - rect.width - CONTEXT_MENU_MARGIN;
  const maxY = window.innerHeight - rect.height - CONTEXT_MENU_MARGIN;

  contextMenuX.value = Math.max(
    CONTEXT_MENU_MARGIN,
    Math.min(contextMenuX.value, maxX),
  );
  contextMenuY.value = Math.max(
    CONTEXT_MENU_MARGIN,
    Math.min(contextMenuY.value, maxY),
  );
}

async function exportToHar() {
  contextMenuVisible.value = false;
  if (selectedRows.value.length === 0) {
    ElMessage.warning("请先勾选会话");
    return;
  }

  exporting.value = true;

  try {
    // 生成默认文件名
    const timestamp = new Date()
      .toISOString()
      .replace(/[:.]/g, "-")
      .slice(0, 19);
    const defaultName = `traffic_${timestamp}.har`;

    // 弹出保存对话框
    const filePath = await save({
      defaultPath: defaultName,
      filters: [{ name: "HAR 文件", extensions: ["har"] }],
    });

    if (!filePath) return; // 用户取消

    // 批量获取详情
    const ids = selectedRows.value.map((r) => r.id);
    const details = await traffic.getDetailsBatch(ids);

    // 过滤掉 null（已过期）并转换
    const entries = details
      .filter((d) => d !== null)
      .map((txn) => transactionToHarEntry(txn));

    if (entries.length === 0) {
      ElMessage.warning("选中的会话已全部过期，无法导出");
      return;
    }

    // 生成 HAR 并写入文件
    const har = generateHarFile(entries);
    await writeTextFile(filePath, JSON.stringify(har, null, 2));

    const expired = ids.length - entries.length;
    ElMessage.success(
      expired > 0
        ? `导出成功：${entries.length} 个会话（${expired} 个已过期）`
        : `导出成功：${entries.length} 个会话`,
    );
  } catch (e) {
    ElMessage.error(`导出失败：${e}`);
  } finally {
    exporting.value = false;
  }
}

async function clear() {
  await traffic.clear();
  tableRef.value?.clearSelection();
  selectedRows.value = [];
}

async function deleteSelected() {
  const ids = selectedRows.value.map((row) => row.id);
  contextMenuVisible.value = false;
  if (ids.length === 0) {
    ElMessage.warning("请先勾选会话");
    return;
  }
  try {
    await traffic.delete(ids);
    tableRef.value?.clearSelection();
    selectedRows.value = [];
    if (selectedId.value && ids.includes(selectedId.value)) {
      drawerVisible.value = false;
      selectedId.value = null;
    }
    ElMessage.success(`已删除 ${ids.length} 个会话`);
  } catch (e) {
    ElMessage.error(`删除失败：${e}`);
  }
}

/** 获取表格滚动容器 DOM，按多个可能的选择器尝试。 */
function getScrollEl() {
  const root = tableRef.value?.$el;
  if (!root) return null;
  // Element Plus v2: el-scrollbar 内部
  return (
    root.querySelector(".el-scrollbar__wrap") ||
    root.querySelector(".el-table__body-wrapper") ||
    root.querySelector(".el-table__body-wrapper-inner")
  );
}

/** 判断表格是否已滚动到底部（允许一定阈值）。 */
function isScrolledToBottom() {
  const el = getScrollEl();
  if (!el) return true;
  return el.scrollHeight - el.scrollTop - el.clientHeight < SCROLL_THRESHOLD;
}

/** 将表格滚动到底部，延迟确保 DOM 已渲染完新行。 */
function scrollToBottom() {
  nextTick(() => {
    const el = getScrollEl();
    if (el) {
      el.scrollTop = el.scrollHeight;
    }
  });
}

/** 监听用户滚动行为：偏离底部时暂停自动滚动，回到底部时恢复。 */
function handleTableScroll() {
  if (isScrolledToBottom()) {
    autoScrollEnabled.value = true;
  } else {
    autoScrollEnabled.value = false;
  }
}

/** 新流量到来时，若处于自动滚动模式则滚到底部。 */
watch(
  () => traffic.list.length,
  () => {
    if (autoScrollEnabled.value) scrollToBottom();
  },
  { flush: "post" },
);

async function handleKeydown(event) {
  if (
    event.ctrlKey &&
    !event.shiftKey &&
    !event.altKey &&
    event.key.toLowerCase() === "w"
  ) {
    event.preventDefault();
    await clear();
    ElMessage.success("已清空会话");
  }
}

let scrollEl = null;

onMounted(async () => {
  document.addEventListener("click", handleClick);
  window.addEventListener("keydown", handleKeydown);
  await traffic.init();
  await refreshProxyStatus();
  // 初始滚动到底部
  scrollToBottom();
  // 监听表格体滚动事件，判断用户是否手动滚动
  nextTick(() => {
    scrollEl = getScrollEl();
    if (scrollEl) {
      scrollEl.addEventListener("scroll", handleTableScroll, { passive: true });
    }
  });
});

onBeforeUnmount(() => {
  document.removeEventListener("click", handleClick);
  window.removeEventListener("keydown", handleKeydown);
  if (scrollEl) scrollEl.removeEventListener("scroll", handleTableScroll);
});
</script>

<style scoped>
.monitor-page {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}

.monitor-panel {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-height: 0;
  overflow: hidden;
  border: 1px solid var(--gm-line);
  border-radius: 10px;
  background: rgba(15, 27, 45, 0.78);
  box-shadow: var(--gm-shadow);
}

.toolbar {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  align-items: center;
  flex-shrink: 0;
  gap: 10px;
  padding: 12px;
  border-bottom: 1px solid var(--gm-line);
}

.monitor-page :deep(.el-table) {
  flex: 1;
  min-height: 0;
}

.monitor-page :deep(.el-checkbox__inner) {
  width: 15px;
  height: 15px;
}

.filter-group,
.batch-actions {
  display: flex;
  min-width: 0;
  align-items: center;
  gap: 10px;
}

.batch-actions {
  justify-content: flex-end;
}

.selection-chip {
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

.selection-chip {
  border-color: var(--gm-line);
  background: rgba(8, 17, 31, 0.56);
  color: var(--gm-muted);
}

.selection-chip.active {
  border-color: rgba(56, 189, 248, 0.44);
  color: var(--gm-cyan);
}

.filter-input {
  width: min(560px, 42vw);
}

.method-filter {
  width: 210px;
}

.method-pill,
.status-pill-cell {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 5px;
  height: 24px;
  min-width: 52px;
  padding: 0 8px;
  border-radius: 7px;
  font-family: "Cascadia Mono", "JetBrains Mono", Consolas, monospace;
  font-size: 12px;
  font-weight: 700;
}

.status-icon {
  width: 13px;
  height: 13px;
  flex: 0 0 auto;
}

.status-icon.spinning {
  animation: status-spin 1s linear infinite;
}

@keyframes status-spin {
  from {
    transform: rotate(0deg);
  }

  to {
    transform: rotate(360deg);
  }
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

.status-ok {
  background: rgba(34, 197, 94, 0.14);
  color: #86efac;
}

.status-pending {
  background: rgba(56, 189, 248, 0.13);
  color: #7dd3fc;
}

.status-warn {
  background: rgba(245, 158, 11, 0.13);
  color: #fcd34d;
}

.status-err {
  background: rgba(248, 113, 113, 0.13);
  color: #fca5a5;
}

.status-none {
  background: rgba(148, 163, 184, 0.12);
  color: var(--gm-muted);
}

.url-cell {
  min-width: 0;
  font-family: "Cascadia Mono", "JetBrains Mono", Consolas, monospace;
  line-height: 1.45;
}

.url-host {
  color: #d9e6f6;
  font-size: 12px;
}

.url-path {
  overflow: hidden;
  color: var(--gm-muted);
  font-size: 12px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.context-menu {
  position: fixed;
  border: 1px solid var(--gm-line);
  border-radius: 8px;
  background: var(--gm-panel-2);
  box-shadow: var(--gm-shadow);
  z-index: 9999;
  padding: 6px 0;
}

.context-menu-item {
  padding: 8px 16px;
  cursor: pointer;
  font-size: inherit;
  color: var(--gm-text);
}

.context-menu-item:hover {
  background: rgba(56, 189, 248, 0.12);
}

.context-menu-item.danger {
  color: #fca5a5;
}

@media (max-width: 980px) {
  .toolbar {
    grid-template-columns: 1fr;
  }

  .filter-group,
  .batch-actions {
    flex-wrap: wrap;
    justify-content: flex-start;
  }

  .filter-input,
  .method-filter {
    width: 100%;
  }
}
</style>
