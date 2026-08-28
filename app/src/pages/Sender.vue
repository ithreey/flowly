<template>
  <div class="sender-page">
    <HistoryPanel class="sender-sidebar" />
    <div class="sender-main">
      <RequestBuilder @send="store.send()" />
      <ResponseViewer />
    </div>
  </div>
</template>

<script setup>
import { useSenderStore } from "../stores/sender";
import HistoryPanel from "../components/HistoryPanel.vue";
import RequestBuilder from "../components/RequestBuilder.vue";
import ResponseViewer from "../components/ResponseViewer.vue";

const store = useSenderStore();
</script>

<style scoped>
.sender-page {
  display: flex;
  height: 100%;
  gap: 0;
}
.sender-sidebar {
  width: 240px;
  flex-shrink: 0;
}
.sender-main {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 16px;
  min-width: 0;
  overflow-y: auto;
}
</style>
