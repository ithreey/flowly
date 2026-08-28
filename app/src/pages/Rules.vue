<template>
  <div class="rules-page">
    <section class="rules-panel">
      <div class="toolbar">
        <div>
          <div class="section-title">规则配置</div>
          <div class="section-subtitle">
            HTTPS 解密域名和规则保存后立即热应用，JSON 模式作为高级入口保留。
          </div>
        </div>
        <div class="spacer" />
        <el-button type="primary" @click="formVisible = true">
          新增规则
        </el-button>
        <el-button @click="mitmVisible = true">HTTPS 解密域名</el-button>
        <el-button @click="openImport">导入 JSON</el-button>
      </div>

      <el-table
        :data="ruleRows"
        size="small"
        height="100%"
        empty-text="暂无规则，点「新增规则」或「导入 JSON」添加。"
      >
        <el-table-column label="启用" width="70">
          <template #default="{ row, $index }">
            <el-switch
              :model-value="row.enabled !== false"
              size="small"
              @change="val => toggleEnabled($index, val)"
            />
          </template>
        </el-table-column>
        <el-table-column label="规则" min-width="160">
          <template #default="{ row }">
            <div class="rule-name">{{ row.name }}</div>
            <div class="rule-meta">{{ row.summary }}</div>
          </template>
        </el-table-column>
        <el-table-column label="匹配" min-width="160">
          <template #default="{ row }">
            <span class="rule-chip">{{ row.matchText }}</span>
          </template>
        </el-table-column>
        <el-table-column label="动作" min-width="150">
          <template #default="{ row }">
            <span class="rule-chip accent">{{ row.actionText }}</span>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="270" align="right">
          <template #default="{ row, $index }">
            <el-button @click="openEdit($index, row)">编辑</el-button>
            <el-button @click="openJsonEdit($index, row)">JSON</el-button>
            <el-button type="danger" plain @click="removeRule($index)">
              删除
            </el-button>
          </template>
        </el-table-column>
      </el-table>
    </section>

    <!-- 导入对话框 -->
    <el-dialog v-model="importVisible" title="导入规则 JSON" width="640px">
      <el-input
        v-model="importText"
        type="textarea"
        :rows="12"
        placeholder="粘贴规则 JSON（数组或单条规则对象均可）"
      />
      <template #footer>
        <el-button @click="importVisible = false">取消</el-button>
        <el-button type="primary" @click="doImport">导入</el-button>
      </template>
    </el-dialog>

    <!-- HTTPS 解密域名设置 -->
    <el-dialog v-model="mitmVisible" title="HTTPS 解密域名" width="560px">
      <div class="mitm-dialog-subtitle">
        配置需要解密的 HTTPS 域名，一行一个，为空则解密所有域名
      </div>
      <el-input
        v-model="mitmText"
        class="mitm-input"
        type="textarea"
        :rows="3"
        placeholder="*.example.com&#10;api.example.com"
      />
      <template #footer>
        <el-button @click="mitmVisible = false">取消</el-button>
        <el-button type="primary" @click="saveMitmHosts">保存</el-button>
      </template>
    </el-dialog>

    <!-- 表单新增/编辑对话框 -->
    <RuleFormDialog
      v-model="formVisible"
      :editing-index="formEditingIndex"
      :initial-json="formInitialJson"
      @submit="onRuleSubmit"
    />

    <!-- 编辑对话框（JSON 模式） -->
    <el-dialog
      v-model="editVisible"
      :title="`编辑规则：${editingName}`"
      width="760px"
    >
      <Codemirror
        v-model="editingJson"
        :style="{
          height: '420px',
          border: '1px solid rgba(126, 161, 196, 0.28)',
          borderRadius: '6px'
        }"
        :extensions="[...extensions, editorTheme]"
        :autofocus="true"
      />
      <template #footer>
        <el-button @click="editVisible = false">取消</el-button>
        <el-button type="primary" @click="confirmEdit">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { computed, ref, onMounted } from "vue";
import { ElMessage } from "element-plus";
import { Codemirror } from "vue-codemirror";
import { json } from "@codemirror/lang-json";
import { EditorView } from "@codemirror/view";
import { useRuleStore } from "../stores/rules";
import RuleFormDialog from "../components/RuleFormDialog.vue";

const extensions = [json()];
const editorTheme = EditorView.theme({
  "&": {
    color: "#d8e5f2",
    backgroundColor: "#0b1627"
  },
  ".cm-content": {
    caretColor: "#55c7ff",
    padding: "12px 0"
  },
  ".cm-gutters": {
    border: "none",
    backgroundColor: "#0b1627",
    color: "#647892",
    paddingRight: "8px"
  },
  ".cm-activeLine, .cm-activeLineGutter": {
    backgroundColor: "rgba(79, 157, 210, 0.08)"
  },
  ".cm-selectionBackground, &.cm-focused .cm-selectionBackground": {
    backgroundColor: "rgba(85, 199, 255, 0.18)"
  },
  ".cm-scroller": {
    fontFamily: "'JetBrains Mono', 'Cascadia Code', Consolas, monospace",
    lineHeight: "1.65"
  }
});
const formVisible = ref(false);
const formEditingIndex = ref(null);
const formInitialJson = ref("");

const ruleStore = useRuleStore();
const importVisible = ref(false);
const importText = ref("");
const editVisible = ref(false);
const mitmVisible = ref(false);
const editingIndex = ref(-1);
const editingName = ref("");
const editingJson = ref("");
const mitmText = ref("");

const ruleRows = computed(() =>
  ruleStore.entries.map(entry => ({
    ...entry,
    ...summarizeRule(entry)
  }))
);

function summarizeRule(entry) {
  try {
    const rule = JSON.parse(entry.json);
    const filters = Array.isArray(rule.filters) ? rule.filters : [];
    const actions = Array.isArray(rule.actions) ? rule.actions : [];
    return {
      summary: rule.enabled === false ? "已停用" : "实时生效",
      matchText: summarizeList(filters, "未配置匹配条件"),
      actionText: summarizeList(actions, "未配置动作")
    };
  } catch {
    return {
      summary: "JSON 暂不可解析",
      matchText: "解析失败",
      actionText: "请打开 JSON 修正"
    };
  }
}

function summarizeList(list, fallback) {
  if (!list.length) return fallback;
  return list
    .slice(0, 2)
    .map(item => item.type || item.kind || item.name || "custom")
    .join(" / ");
}

/** 每次规则变更后立即保存并热应用（实时生效）。 */
async function applyNow() {
  try {
    await ruleStore.save();
  } catch (e) {
    ElMessage.error(String(e));
  }
}

async function saveMitmHosts() {
  ruleStore.mitmHosts = mitmText.value
    .split(/\r?\n/)
    .map(item => item.trim())
    .filter(Boolean);
  await applyNow();
  mitmVisible.value = false;
}

/** 表单新增/编辑：把组装好的规则对象转成 JSON 条目加入列表（或替换对应下标）。 */
function onRuleSubmit({ rule, editingIndex: idx }) {
  const json = JSON.stringify(rule, null, 2);
  const entry = {
    name: rule.name,
    enabled: rule.enabled !== false,
    json
  };
  if (idx == null) {
    ruleStore.entries.push(entry);
  } else {
    ruleStore.entries[idx] = entry;
  }
  applyNow();
}

/** 切换规则启用状态：同步更新条目里的 JSON，并立即应用。 */
function toggleEnabled(index, val) {
  const entry = ruleStore.entries[index];
  entry.enabled = val;
  try {
    const obj = JSON.parse(entry.json);
    obj.enabled = val;
    entry.json = JSON.stringify(obj, null, 2);
  } catch {
    // JSON 暂不可解析时只更新开关状态，保存时由后端校验。
  }
  applyNow();
}

/** 表单编辑：打开表单对话框并回填该规则。 */
function openEdit(index, row) {
  formEditingIndex.value = index;
  formInitialJson.value = row.json;
  formVisible.value = true;
}

/** JSON 高级编辑（表单不覆盖的复杂结构）。 */
function openJsonEdit(index, row) {
  editingIndex.value = index;
  editingName.value = row.name;
  editingJson.value = row.json;
  editVisible.value = true;
}

function confirmEdit() {
  if (editingIndex.value >= 0) {
    ruleStore.entries[editingIndex.value].json = editingJson.value;
    applyNow();
  }
  editVisible.value = false;
}

function removeRule(index) {
  ruleStore.entries.splice(index, 1);
  applyNow();
}

function openImport() {
  importText.value = "";
  importVisible.value = true;
}

async function doImport() {
  try {
    await ruleStore.importJson(importText.value);
    importVisible.value = false;
    ElMessage.success("导入成功");
    applyNow();
  } catch (e) {
    ElMessage.error(String(e));
  }
}

onMounted(() => {
  ruleStore
    .load()
    .then(() => {
      mitmText.value = ruleStore.mitmHosts.join("\n");
    })
    .catch(e => ElMessage.error(String(e)));
});
</script>

<style scoped>
.rules-page {
  display: flex;
  height: 100%;
  min-height: 0;
}

.rules-panel {
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
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 12px;
  border-bottom: 1px solid var(--gm-line);
}

.mitm-dialog-subtitle {
  margin-bottom: 10px;
  color: var(--gm-muted);
  font-size: 12px;
  line-height: 1.5;
}

.mitm-input :deep(.el-textarea__inner) {
  height: 78px;
  min-height: 78px !important;
  max-height: 78px;
  overflow-y: auto;
  resize: none;
}

.spacer {
  flex: 1;
}

.section-title {
  color: var(--gm-text);
  font-size: 14px;
  font-weight: 700;
}

.section-subtitle {
  margin-top: 3px;
  color: var(--gm-muted);
  font-size: 12px;
}

.rule-name {
  color: var(--gm-text);
  font-weight: 700;
}

.rule-meta {
  margin-top: 4px;
  color: var(--gm-muted);
  font-size: 12px;
}

.rule-chip {
  display: inline-flex;
  max-width: 100%;
  align-items: center;
  height: 26px;
  padding: 0 9px;
  overflow: hidden;
  border: 1px solid var(--gm-line);
  border-radius: 7px;
  background: rgba(8, 17, 31, 0.56);
  color: var(--gm-muted);
  font-size: 12px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.rule-chip.accent {
  border-color: rgba(56, 189, 248, 0.28);
  color: var(--gm-cyan);
}
</style>
