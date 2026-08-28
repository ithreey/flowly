<template>
  <el-dialog
    :model-value="modelValue"
    :title="editingIndex == null ? '新增规则' : '编辑规则'"
    width="700px"
    @update:model-value="$emit('update:modelValue', $event)"
  >
    <el-form label-width="90px">
      <el-form-item label="规则名称">
        <el-input v-model="form.name" placeholder="规则名称" />
      </el-form-item>
      <el-form-item label="启用">
        <el-switch v-model="form.enabled" />
      </el-form-item>

      <el-form-item label="筛选条件">
        <div class="list">
          <div v-for="(f, i) in form.filters" :key="i" class="row">
            <el-select v-model="f.type" size="small" class="type-select">
              <el-option value="all" label="全部" />
              <el-option value="domain" label="域名" />
              <el-option value="domainKeyword" label="域名包含" />
              <el-option value="domainPrefix" label="域名前缀" />
              <el-option value="domainSuffix" label="域名后缀" />
              <el-option value="urlContains" label="URL 包含" />
              <el-option value="urlRegex" label="URL 正则" />
            </el-select>
            <el-input
              v-if="f.type !== 'all'"
              v-model="f.value"
              size="small"
              class="value-input"
              :placeholder="f.type === 'urlRegex' ? '正则表达式' : '匹配值'"
            />
            <el-button size="small" text type="danger" @click="removeFilter(i)">
              删除
            </el-button>
          </div>
          <el-button size="small" plain @click="addFilter">
            + 添加筛选条件
          </el-button>
        </div>
      </el-form-item>

      <el-form-item label="动作">
        <div class="list">
          <div v-for="(a, i) in form.actions" :key="i" class="row">
            <el-select v-model="a.type" size="small" class="type-select">
              <el-option
                v-for="item in visibleActionTypes"
                :key="item.value"
                :value="item.value"
                :label="item.label"
              />
            </el-select>
            <el-input
              v-if="a.type === 'redirect'"
              v-model="a.url"
              size="small"
              class="value-input"
              placeholder="目标 URL"
            />
            <template
              v-if="
                a.type === 'setRequestHeader' || a.type === 'setResponseHeader'
              "
            >
              <el-input
                v-model="a.key"
                size="small"
                class="value-input"
                placeholder="Header 名称"
              />
              <el-input
                v-model="a.value"
                size="small"
                class="value-input"
                placeholder="Header 值"
              />
            </template>
            <template
              v-if="
                a.type === 'modifyRequestBody' ||
                a.type === 'modifyResponseBody'
              "
            >
              <el-input
                v-model="a.origin"
                size="small"
                class="value-input"
                placeholder="原文本"
              />
              <el-input
                v-model="a.new"
                size="small"
                class="value-input"
                placeholder="替换为"
              />
            </template>
            <el-button size="small" text type="danger" @click="removeAction(i)">
              删除
            </el-button>
          </div>
          <el-button size="small" plain @click="addAction"
            >+ 添加动作</el-button
          >
        </div>
      </el-form-item>
    </el-form>

    <template #footer>
      <el-button @click="$emit('update:modelValue', false)">取消</el-button>
      <el-button type="primary" @click="submit">确定</el-button>
    </template>
  </el-dialog>
</template>

<script setup>
import { reactive, watch } from "vue";
import {
  createDefaultAction,
  createDefaultForm,
  parseActions,
  serializeFormRule,
  visibleActionTypes,
} from "../utils/rule-form";

const props = defineProps({
  modelValue: { type: Boolean, default: false },
  /** null=新增；非 null=编辑对应下标的规则 */
  editingIndex: { type: Number, default: null },
  /** 编辑时传入该条规则的 JSON 文本，用于回填表单 */
  initialJson: { type: String, default: "" },
});
const emit = defineEmits(["update:modelValue", "submit"]);

const form = reactive(createDefaultForm());

// 把规则 JSON 反向解析为表单结构。
function parseFilters(filters) {
  if (!Array.isArray(filters)) filters = [filters];
  return filters.map((f) => {
    if (f && typeof f === "object") {
      const type = Object.keys(f)[0];
      return { type, value: type === "all" ? "" : String(f[type] ?? "") };
    }
    return { type: "all", value: "" };
  });
}

// 每次打开时填充表单（新增用默认，编辑回填）。
watch(
  () => props.modelValue,
  (v) => {
    if (!v) return;
    const base = createDefaultForm();
    if (props.editingIndex != null && props.initialJson) {
      try {
        const r = JSON.parse(props.initialJson);
        base.name = r.name ?? base.name;
        base.enabled = r.enabled !== false;
        base.filters = parseFilters(r.filters);
        base.actions = parseActions(r.actions);
      } catch {
        // 解析失败时使用默认表单
      }
    }
    Object.assign(form, base);
  },
);

function addFilter() {
  form.filters.push({ type: "all", value: "" });
}
function removeFilter(i) {
  form.filters.splice(i, 1);
}
function addAction() {
  form.actions.push(createDefaultAction());
}
function removeAction(i) {
  form.actions.splice(i, 1);
}

function submit() {
  emit("submit", {
    rule: serializeFormRule(form),
    editingIndex: props.editingIndex,
  });
  emit("update:modelValue", false);
}
</script>

<style scoped>
.list {
  width: 100%;
}

.row {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 8px;
}

.type-select {
  width: 130px;
  flex-shrink: 0;
}

.value-input {
  flex: 1;
}
</style>
