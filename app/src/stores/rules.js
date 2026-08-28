import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";

export const useRuleStore = defineStore("rules", {
  state: () => ({
    entries: [], // [{ name, json }]
    mitmHosts: [],
  }),
  actions: {
    async load() {
      const [entries, config] = await Promise.all([
        invoke("rules_list"),
        invoke("config_get"),
      ]);
      this.entries = entries;
      this.mitmHosts = config.mitmHosts || [];
    },
    async save() {
      return await invoke("rules_save", {
        entries: this.entries,
        mitmHosts: this.mitmHosts,
      });
    },
    async importJson(json) {
      this.entries = await invoke("rules_import_json", { json });
    },
  },
});
