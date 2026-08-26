import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";

export const useRuleStore = defineStore("rules", {
  state: () => ({
    entries: [], // [{ name, json }]
  }),
  actions: {
    async load() {
      this.entries = await invoke("rules_list");
    },
    async save() {
      return await invoke("rules_save", { entries: this.entries });
    },
    async importJson(json) {
      this.entries = await invoke("rules_import_json", { json });
    },
  },
});
