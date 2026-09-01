import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

const MAX_ITEMS = 500;

export const useTrafficStore = defineStore("traffic", {
  state: () => ({
    // 摘要列表，按时间正序排列，最新的在后。
    list: [],
    listening: false,
  }),
  actions: {
    async init() {
      if (this.listening) return;
      this.listening = true;
      await listen("traffic://batch", (event) => {
        const batch = event.payload || [];
        for (const item of batch) {
          const index = this.list.findIndex((current) => current.id === item.id);
          if (index >= 0) {
            this.list.splice(index, 1, item);
          } else {
            this.list.push(item);
          }
        }
        if (this.list.length > MAX_ITEMS) {
          this.list.splice(0, this.list.length - MAX_ITEMS);
        }
      });
    },
    async getDetail(id) {
      return await invoke("traffic_get", { id });
    },
    async getDetailMeta(id) {
      return await invoke("traffic_get_meta", { id });
    },
    async getBody(id, kind) {
      return await invoke("traffic_get_body", { id, kind });
    },
    /** 批量获取完整事务详情（按 id 数组），返回数组，顺序与 ids 一致，缺失的为 null。 */
    async getDetailsBatch(ids) {
      return await invoke("traffic_get_batch", { ids });
    },
    async replay(id) {
      return await invoke("traffic_replay", { id });
    },
    async clear() {
      await invoke("traffic_clear");
      this.list = [];
    },
    async delete(ids) {
      await invoke("traffic_delete", { ids });
      const idSet = new Set(ids);
      this.list = this.list.filter((item) => !idSet.has(item.id));
    },
  },
});
