<template>
  <div class="app-settings-page">
    <section class="settings-panel">
      <div class="panel-head">
        <div>
          <div class="panel-title">应用设置</div>
          <div class="panel-subtitle">调整工作台显示密度和本地界面偏好。</div>
        </div>
      </div>

      <el-form label-width="140px" @submit.prevent>
        <el-form-item label="字体大小">
          <el-input-number
            v-model="fontSize"
            :min="MIN_FONT_SIZE"
            :max="MAX_FONT_SIZE"
            :step="1"
            controls-position="right"
            @change="updateFontSize"
          />
          <span class="tip">单位 px，调整后立即生效</span>
          <el-button class="reset-btn" plain @click="resetFontSize">
            恢复默认
          </el-button>
        </el-form-item>
      </el-form>
    </section>
  </div>
</template>

<script setup>
import { ref } from "vue";
import {
  DEFAULT_FONT_SIZE,
  MAX_FONT_SIZE,
  MIN_FONT_SIZE,
  loadFontSize,
  saveFontSize,
} from "../utils/ui-settings";

const fontSize = ref(loadFontSize());

function updateFontSize(value) {
  fontSize.value = saveFontSize(value);
}

function resetFontSize() {
  fontSize.value = saveFontSize(DEFAULT_FONT_SIZE);
}
</script>

<style scoped>
.app-settings-page {
  height: 100%;
}

.settings-panel {
  max-width: 760px;
  border: 1px solid var(--gm-line);
  border-radius: 10px;
  background: rgba(15, 27, 45, 0.78);
  box-shadow: var(--gm-shadow);
}

.panel-head {
  padding: 14px 16px;
  border-bottom: 1px solid var(--gm-line);
}

.panel-title {
  color: var(--gm-text);
  font-size: 15px;
  font-weight: 700;
}

.panel-subtitle {
  margin-top: 4px;
  color: var(--gm-muted);
  font-size: 12px;
}

:deep(.el-form) {
  padding: 16px;
}

.tip {
  margin-left: 8px;
  color: var(--gm-muted);
  font-size: 12px;
  white-space: nowrap;
}

.reset-btn {
  margin-left: 8px;
}

:deep(.el-form-item__label) {
  white-space: nowrap;
  color: var(--gm-muted);
}

:deep(.el-form-item__content) {
  flex-wrap: nowrap;
}
</style>
