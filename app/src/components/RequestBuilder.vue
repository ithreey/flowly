<template>
  <div class="request-builder">
    <!-- URL 行 -->
    <div class="url-bar">
      <el-select
        v-model="store.method"
        size="small"
        style="width: 100px"
        class="method-select"
      >
        <el-option v-for="m in methods" :key="m" :value="m" :label="m" />
      </el-select>
      <el-input
        v-model="store.url"
        placeholder="输入 URL 或粘贴 cURL 命令..."
        size="small"
        class="url-input"
        @paste="onPaste"
      />
      <el-checkbox v-model="store.throughProxy" size="small" class="proxy-check">经过代理</el-checkbox>
      <el-button type="primary" size="small" :loading="store.sending" @click="$emit('send')">
        Send
      </el-button>
    </div>

    <!-- Tab 面板 -->
    <el-tabs v-model="activeTab" class="request-tabs">
      <el-tab-pane label="Params" name="params">
        <KeyValueTable v-model="store.params" add-label="+ 添加参数" />
      </el-tab-pane>
      <el-tab-pane label="Headers" name="headers">
        <KeyValueTable v-model="store.headers" add-label="+ 添加请求头" />
      </el-tab-pane>
      <el-tab-pane label="Body" name="body">
        <div class="body-type-row">
          <el-radio-group v-model="store.bodyType" size="small">
            <el-radio-button value="none">none</el-radio-button>
            <el-radio-button value="x-www-form-urlencoded">x-www-form-urlencoded</el-radio-button>
            <el-radio-button value="raw">raw</el-radio-button>
          </el-radio-group>
          <template v-if="store.bodyType === 'raw'">
            <el-select
              v-model="store.bodyRawFormat"
              size="small"
              style="width: 80px; margin-left: 8px"
            >
              <el-option value="Text" label="Text" />
              <el-option value="JSON" label="JSON" />
              <el-option value="XML" label="XML" />
              <el-option value="HTML" label="HTML" />
            </el-select>
            <el-checkbox v-model="prettyEnabled" size="small" style="margin-left: 12px">Pretty</el-checkbox>
          </template>
        </div>
        <div v-if="store.bodyType === 'none'" class="body-empty">此请求没有 Body</div>
        <KeyValueTable
          v-else-if="store.bodyType === 'x-www-form-urlencoded'"
          v-model="store.params"
          add-label="+ 添加字段"
        />
        <div v-else-if="store.bodyType === 'raw'" class="body-editor">
          <codemirror
            v-if="prettyEnabled"
            :model-value="prettyBody"
            class="body-codemirror"
            :extensions="bodyExtensions"
            @update:model-value="onPrettyUpdate"
          />
          <codemirror
            v-else
            v-model="store.body"
            class="body-codemirror"
          />
        </div>
      </el-tab-pane>
      <el-tab-pane label="Auth" name="auth">
        <div class="body-empty">即将支持</div>
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup>
import { ref, computed } from "vue";
import { ElMessage } from "element-plus";
import { Codemirror } from "vue-codemirror";
import { json } from "@codemirror/lang-json";
import { xml } from "@codemirror/lang-xml";
import { useSenderStore } from "../stores/sender";
import { parseCurl } from "../utils/curl";
import KeyValueTable from "./KeyValueTable.vue";

defineEmits(["send"]);

const store = useSenderStore();
const activeTab = ref("params");
const prettyEnabled = ref(true);
const methods = ["GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS", "HEAD"];

const prettyBody = computed(() => {
  const body = store.body || "";
  if (store.bodyRawFormat === "JSON") {
    try {
      return JSON.stringify(JSON.parse(body), null, 2);
    } catch {
      return body;
    }
  }
  return body;
});

function onPrettyUpdate(val) {
  // Pretty 模式编辑后同步回 store.body
  store.body = val;
}

const bodyExtensions = computed(() => {
  const exts = [];
  if (store.bodyRawFormat === "JSON") exts.push(json());
  else if (store.bodyRawFormat === "XML" || store.bodyRawFormat === "HTML")
    exts.push(xml());
  return exts;
});

function onPaste(event) {
  const text = event.clipboardData?.getData("text") || "";
  if (!text.trim().startsWith("curl")) return;

  const parsed = parseCurl(text);
  if (!parsed) {
    ElMessage.warning("无法解析 cURL 命令");
    return;
  }

  event.preventDefault();
  store.method = parsed.method;
  store.url = parsed.url;
  if (parsed.headers.length) store.headers = parsed.headers;
  if (parsed.body) {
    store.body = parsed.body;
    store.bodyType = parsed.bodyType;
    if (
      parsed.bodyType === "raw" &&
      (parsed.body.startsWith("{") || parsed.body.startsWith("["))
    ) {
      store.bodyRawFormat = "JSON";
    }
  }
}
</script>

<style scoped>
.request-builder {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.url-bar {
  display: flex;
  gap: 8px;
  align-items: center;
  flex-shrink: 0;
}
.url-input {
  flex: 1;
}
.method-select :deep(.el-select__wrapper) {
  background: rgba(15, 27, 45, 0.9);
  box-shadow: 0 0 0 1px var(--gm-line) inset;
}
.proxy-check {
  flex-shrink: 0;
  margin: 0 4px;
}
.request-tabs :deep(.el-tabs__header) {
  margin-bottom: 8px;
}
.body-type-row {
  display: flex;
  align-items: center;
  margin-bottom: 8px;
  flex-shrink: 0;
}
.body-empty {
  color: var(--gm-subtle);
  font-size: 12px;
  padding: 24px;
  text-align: center;
}
.body-editor {
  border: 1px solid var(--gm-line);
  border-radius: 4px;
  overflow: hidden;
}
.body-editor-toolbar {
  padding: 4px 8px;
  border-bottom: 1px solid var(--gm-line);
  background: rgba(15, 27, 45, 0.5);
}
.body-codemirror :deep(.cm-editor) {
  height: 180px;
}
.body-codemirror :deep(.cm-scroller) {
  overflow: auto;
}
.body-editor-toolbar {
  padding: 4px 8px;
  border-bottom: 1px solid var(--gm-line);
  background: rgba(15, 27, 45, 0.5);
}
</style>
