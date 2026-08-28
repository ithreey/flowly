<template>
  <div class="history-panel">
    <div class="history-header">
      <span class="history-title">历史记录</span>
      <el-button
        text
        size="small"
        @click="confirmClear"
        :disabled="!store.history.length"
      >
        清空
      </el-button>
    </div>
    <el-input
      v-model="searchText"
      placeholder="搜索 URL..."
      size="small"
      clearable
      class="history-search"
    />
    <div class="history-list">
      <template v-for="group in filteredGroups" :key="group.label">
        <div class="history-group-label">{{ group.label }}</div>
        <div
          v-for="item in group.items"
          :key="item.id"
          class="history-item"
          :class="{ active: selectedId === item.id }"
          @click="selectEntry(item)"
          @contextmenu.prevent="showContextMenu($event, item)"
        >
          <span
            class="method-tag"
            :class="'method-' + item.method.toLowerCase()"
          >
            {{ item.method }}
          </span>
          <span class="item-url" :title="item.url">{{
            extractPath(item.url)
          }}</span>
        </div>
      </template>
      <div v-if="filteredGroups.length === 0" class="history-empty">
        无记录
      </div>
    </div>

    <!-- 右键菜单 -->
    <div
      v-if="ctxMenu.visible"
      class="history-context-menu"
      :style="{ left: ctxMenu.x + 'px', top: ctxMenu.y + 'px' }"
    >
      <div class="ctx-item" @click="copyAsCurl">复制为 cURL</div>
      <div class="ctx-item danger" @click="deleteEntry">删除</div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onBeforeUnmount } from "vue";
import { ElMessageBox, ElMessage } from "element-plus";
import { useSenderStore } from "../stores/sender";
import { toCurl } from "../utils/curl";

const store = useSenderStore();
const searchText = ref("");
const selectedId = ref(null);
const ctxMenu = ref({ visible: false, x: 0, y: 0, item: null });

onMounted(() => {
  store.loadHistory();
  document.addEventListener("click", hideContextMenu);
});

onBeforeUnmount(() => {
  document.removeEventListener("click", hideContextMenu);
});

function hideContextMenu() {
  ctxMenu.value.visible = false;
}

const filteredGroups = computed(() => {
  const items = store.history.filter((h) =>
    searchText.value
      ? h.url.toLowerCase().includes(searchText.value.toLowerCase())
      : true
  );
  const now = new Date();
  const todayStart = new Date(
    now.getFullYear(),
    now.getMonth(),
    now.getDate()
  ).getTime();
  const yesterdayStart = todayStart - 86400000;

  const groups = [];
  const today = [];
  const yesterday = [];
  const earlier = [];

  for (const item of items) {
    if (item.timestamp >= todayStart) today.push(item);
    else if (item.timestamp >= yesterdayStart) yesterday.push(item);
    else earlier.push(item);
  }

  if (today.length) groups.push({ label: "今天", items: today });
  if (yesterday.length) groups.push({ label: "昨天", items: yesterday });
  if (earlier.length) groups.push({ label: "更早", items: earlier });
  return groups;
});

function selectEntry(item) {
  selectedId.value = item.id;
  store.loadFromHistory(item);
}

function extractPath(url) {
  try {
    const u = new URL(url);
    return u.pathname === "/" ? u.host : u.host + u.pathname;
  } catch {
    return url.slice(0, 40);
  }
}

function showContextMenu(event, item) {
  ctxMenu.value = { visible: true, x: event.clientX, y: event.clientY, item };
}

async function copyAsCurl() {
  const item = ctxMenu.value.item;
  if (!item) return;
  const curl = toCurl({
    method: item.method,
    url: item.url,
    headers: (item.headers || []).map(([key, value]) => ({
      key,
      value,
      enabled: true,
    })),
    body: item.body
      ? new TextDecoder().decode(new Uint8Array(item.body))
      : "",
    bodyType: item.bodyType || (item.body ? "raw" : "none"),
    formRows: item.formRows || [],
  });
  try {
    await navigator.clipboard.writeText(curl);
    ElMessage.success("已复制为 cURL");
  } catch (e) {
    ElMessage.error("复制失败: " + e);
  }
  ctxMenu.value.visible = false;
}

async function deleteEntry() {
  const item = ctxMenu.value.item;
  if (!item) return;
  await store.deleteHistory(item.id);
  if (selectedId.value === item.id) selectedId.value = null;
  ctxMenu.value.visible = false;
}

async function confirmClear() {
  try {
    await ElMessageBox.confirm("确定清空所有历史记录？", "清空历史", {
      type: "warning",
    });
    await store.clearHistory();
    selectedId.value = null;
  } catch {
    // 用户取消
  }
}
</script>

<style scoped>
.history-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  border-right: 1px solid var(--gm-line);
  background: rgba(15, 27, 45, 0.78);
}
.history-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 12px 8px;
}
.history-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--gm-text);
}
.history-search {
  padding: 0 12px 8px;
}
.history-list {
  flex: 1;
  overflow-y: auto;
  padding: 0 8px;
}
.history-group-label {
  font-size: 11px;
  color: var(--gm-subtle);
  padding: 8px 4px 4px;
  border-bottom: 1px solid var(--gm-line);
  margin-bottom: 4px;
}
.history-item {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
  padding: 6px 8px;
  border-radius: 4px;
  cursor: pointer;
  transition: background 0.15s;
}
.history-item:hover {
  background: rgba(56, 189, 248, 0.08);
}
.history-item.active {
  background: rgba(56, 189, 248, 0.15);
}
.method-tag {
  font-size: 10px;
  font-weight: 700;
  padding: 1px 5px;
  border-radius: 3px;
  flex-shrink: 0;
}
.method-get {
  background: rgba(34, 197, 94, 0.15);
  color: #22c55e;
}
.method-post {
  background: rgba(56, 189, 248, 0.15);
  color: #38bdf8;
}
.method-put {
  background: rgba(245, 158, 11, 0.15);
  color: #f59e0b;
}
.method-delete {
  background: rgba(248, 113, 113, 0.15);
  color: #f87171;
}
.method-patch {
  background: rgba(168, 85, 247, 0.15);
  color: #a855f7;
}
.method-options,
.method-head {
  background: rgba(148, 163, 184, 0.15);
  color: #94a3b8;
}
.item-url {
  font-size: 12px;
  color: var(--gm-text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
  min-width: 0;
}
.history-empty {
  text-align: center;
  color: var(--gm-subtle);
  padding: 24px;
  font-size: 12px;
}
.history-context-menu {
  position: fixed;
  z-index: 3000;
  background: #1a2740;
  border: 1px solid var(--gm-line);
  border-radius: 4px;
  padding: 4px 0;
  min-width: 120px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
}
.ctx-item {
  padding: 6px 12px;
  font-size: 12px;
  color: var(--gm-text);
  cursor: pointer;
}
.ctx-item:hover {
  background: rgba(56, 189, 248, 0.1);
}
.ctx-item.danger {
  color: #f87171;
}
</style>
