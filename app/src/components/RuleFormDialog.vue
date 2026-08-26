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
      <el-form-item label="MITM 域名">
        <el-input
          v-model="form.mitm"
          placeholder="如 *.baidu.com（需要 MITM 的 HTTPS 域名，可留空）"
        />
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
              <el-option value="reject" label="拒绝(502)" />
              <el-option value="redirect" label="重定向" />
              <el-option value="intercept" label="拦截确认" />
              <el-option value="logReq" label="记录请求" />
              <el-option value="logRes" label="记录响应" />
              <el-option value="modifyBody" label="修改响应体" />
            </el-select>
            <el-input
              v-if="a.type === 'redirect'"
              v-model="a.url"
              size="small"
              class="value-input"
              placeholder="目标 URL"
            />
            <template v-if="a.type === 'modifyBody'">
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
          <el-button size="small" plain @click="addAction">+ 添加动作</el-button>
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

const props = defineProps({
  modelValue: { type: Boolean, default: false },
  /** null=新增；非 null=编辑对应下标的规则 */
  editingIndex: { type: Number, default: null },
  /** 编辑时传入该条规则的 JSON 文本，用于回填表单 */
  initialJson: { type: String, default: "" },
});
const emit = defineEmits(["update:modelValue", "submit"]);

function defaultForm() {
  return {
    name: "new rule",
    enabled: true,
    mitm: "",
    filters: [{ type: "all", value: "" }],
    actions: [{ type: "logReq" }],
  };
}

const form = reactive(defaultForm());

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

function parseActions(actions) {
  if (!Array.isArray(actions)) actions = [actions];
  return actions.map((a) => {
    if (typeof a === "string") return { type: a };
    if (a && typeof a === "object") {
      if (a.redirect != null) return { type: "redirect", url: String(a.redirect) };
      if (a.modifyResponse && a.modifyResponse.body) {
        const b = a.modifyResponse.body;
        if (typeof b === "string") {
          return { type: "modifyBody", origin: "", new: b };
        }
        return {
          type: "modifyBody",
          origin: String(b.origin ?? ""),
          new: String(b.new ?? ""),
        };
      }
    }
    return { type: "logReq" };
  });
}

// 每次打开时填充表单（新增用默认，编辑回填）。
watch(
  () => props.modelValue,
  (v) => {
    if (!v) return;
    const base = defaultForm();
    if (props.editingIndex != null && props.initialJson) {
      try {
        const r = JSON.parse(props.initialJson);
        base.name = r.name ?? base.name;
        base.enabled = r.enabled !== false;
        base.mitm = r.mitmList ?? r.mitm ?? "";
        base.filters = parseFilters(r.filters);
        base.actions = parseActions(r.actions);
      } catch {
        // 解析失败时使用默认表单
      }
    }
    Object.assign(form, base);
  }
);

function addFilter() {
  form.filters.push({ type: "all", value: "" });
}
function removeFilter(i) {
  form.filters.splice(i, 1);
}
function addAction() {
  form.actions.push({ type: "logReq" });
}
function removeAction(i) {
  form.actions.splice(i, 1);
}

/** 把表单组装成规则的 JSON 对象。 */
function buildRule() {
  const rule = {
    name: form.name.trim() || "new rule",
    enabled: form.enabled,
    filters: form.filters.map((f) =>
      f.type === "all" ? { all: null } : { [f.type]: f.value }
    ),
    actions: form.actions.map((a) => {
      switch (a.type) {
        case "reject":
          return "reject";
        case "redirect":
          return { redirect: a.url };
        case "intercept":
          return "intercept";
        case "logReq":
          return "logReq";
        case "logRes":
          return "logRes";
        case "modifyBody":
          return { modifyResponse: { body: { origin: a.origin, new: a.new } } };
      }
    }),
  };
  if (form.mitm.trim()) rule.mitmList = form.mitm.trim();
  return rule;
}

function submit() {
  emit("submit", { rule: buildRule(), editingIndex: props.editingIndex });
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
