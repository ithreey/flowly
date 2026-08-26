<template>
  <el-dialog
    v-model="visible"
    title="🚦 请求拦截"
    width="600px"
    :close-on-click-modal="false"
    :close-on-press-escape="false"
    :show-close="false"
  >
    <div v-if="current">
      <p class="req-line">
        <el-tag size="small">{{ current.method }}</el-tag>
        <span class="url">{{ current.url }}</span>
      </p>
      <el-input
        type="textarea"
        :rows="6"
        :model-value="headersText"
        readonly
        class="headers"
      />
      <div class="actions">
        <el-button type="success" @click="decide({ type: 'allow' })">
          放行
        </el-button>
        <el-button type="danger" @click="decide({ type: 'reject' })">
          拒绝
        </el-button>
        <div class="redirect">
          <el-input
            v-model="redirectUrl"
            placeholder="重定向 URL，如 https://example.com"
            size="small"
          />
          <el-button type="primary" @click="doRedirect">重定向</el-button>
        </div>
      </div>
    </div>
  </el-dialog>
</template>

<script setup>
import { ref, computed, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

const visible = ref(false);
const current = ref(null);
const redirectUrl = ref("");

const headersText = computed(() =>
  (current.value?.headers || []).map(([k, v]) => `${k}: ${v}`).join("\n")
);

function show(payload) {
  current.value = payload;
  redirectUrl.value = "";
  visible.value = true;
}

async function decide(decision) {
  if (!current.value) return;
  const id = current.value.id;
  visible.value = false;
  try {
    await invoke("intercept_decide", { id, decision });
  } catch (e) {
    console.error("intercept_decide failed:", e);
  }
}

function doRedirect() {
  if (!redirectUrl.value) return;
  decide({ type: "redirect", url: redirectUrl.value });
}

onMounted(async () => {
  await listen("intercept://pending", (event) => show(event.payload));
  // 超时提示（可选）：日志记录即可，无需弹窗。
  await listen("intercept://timeout", (event) => {
    console.warn("拦截超时已放行:", event.payload);
  });
});
</script>

<style scoped>
.req-line {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 0 0 8px;
  word-break: break-all;
}

.url {
  font-family: Consolas, "Courier New", monospace;
  font-size: 13px;
}

.headers {
  font-family: Consolas, "Courier New", monospace;
}

.actions {
  margin-top: 12px;
  display: flex;
  align-items: center;
  gap: 8px;
}

.redirect {
  flex: 1;
  display: flex;
  gap: 8px;
}
</style>
