<template>
  <div class="kv-table">
    <div v-for="(row, i) in modelValue" :key="i" class="kv-row">
      <el-checkbox v-model="row.enabled" size="small" />
      <el-input v-model="row.key" placeholder="Key" size="small" class="kv-input" />
      <el-input
        v-model="row.value"
        placeholder="Value"
        size="small"
        class="kv-input"
      />
      <el-button text size="small" @click="removeRow(i)" class="kv-delete"
        >&times;</el-button
      >
    </div>
    <el-button size="small" text @click="addRow">{{ addLabel }}</el-button>
  </div>
</template>

<script setup>
const props = defineProps({
  modelValue: { type: Array, default: () => [] },
  addLabel: { type: String, default: "+ 添加" },
});
const emit = defineEmits(["update:modelValue"]);

function addRow() {
  emit("update:modelValue", [
    ...props.modelValue,
    { key: "", value: "", enabled: true },
  ]);
}

function removeRow(index) {
  const next = [...props.modelValue];
  next.splice(index, 1);
  emit("update:modelValue", next);
}
</script>

<style scoped>
.kv-table {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.kv-row {
  display: flex;
  align-items: center;
  gap: 6px;
}
.kv-input {
  flex: 1;
}
.kv-delete {
  color: var(--gm-subtle);
  font-size: 16px;
  padding: 0 4px;
}
.kv-delete:hover {
  color: #f87171;
}
</style>
