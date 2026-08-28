import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";

const SKIPPED_REQUEST_HEADERS = new Set([
  "if-none-match",
  "if-modified-since",
]);

export const useSenderStore = defineStore("sender", {
  state: () => ({
    // 当前请求
    method: "GET",
    url: "",
    params: [],
    formRows: [],
    headers: [{ key: "Accept", value: "*/*", enabled: true }],
    bodyType: "none",
    body: "",
    bodyRawFormat: "Text",

    // 当前响应
    response: null,
    sending: false,
    error: null,

    // 历史记录
    history: [],
  }),

  actions: {
    setRequest(req) {
      this.method = req.method || "GET";
      this.url = req.url || "";
      this.params = req.params || [];
      this.formRows = req.formRows || [];
      this.headers = req.headers || [
        { key: "Accept", value: "*/*", enabled: true },
      ];
      this.bodyType = req.bodyType || "none";
      this.body = req.body || "";
      this.bodyRawFormat = req.bodyRawFormat || "Text";
    },

    async send() {
      this.sending = true;
      this.error = null;
      this.response = null;

      const finalUrl = this._buildUrl();
      const reqHeaders = this._buildHeaders();
      const bodyBytes = this._buildBodyBytes();

      try {
        const resp = await invoke("send_request", {
          method: this.method,
          url: finalUrl,
          headers: reqHeaders,
          body: bodyBytes,
        });

        this.response = {
          url: finalUrl,
          status: resp.status,
          statusText: resp.statusText,
          headers: resp.headers,
          bodyBytes: resp.body, // 保持原始字节数组
          durationMs: resp.durationMs,
          size: resp.body.length,
        };

        // 保存到历史（只存请求，不存响应）
        await invoke("history_save", {
          entry: {
            id: 0,
            method: this.method,
            url: finalUrl,
            params: this.params,
            formRows: this.formRows,
            headers: reqHeaders,
            bodyType: this.bodyType,
            bodyRawFormat: this.bodyRawFormat,
            body: bodyBytes,
            timestamp: Date.now(),
          },
        });
        await this.loadHistory();
      } catch (e) {
        this.error = String(e);
      } finally {
        this.sending = false;
      }
    },

    async loadHistory() {
      try {
        this.history = await invoke("history_list");
      } catch (e) {
        console.error("加载历史记录失败:", e);
      }
    },

    async clearHistory() {
      await invoke("history_clear");
      this.history = [];
    },

    async deleteHistory(id) {
      await invoke("history_delete", { id });
      this.history = this.history.filter((h) => h.id !== id);
    },

    loadFromHistory(entry) {
      this.setRequest({
        method: entry.method,
        url: entry.url,
        params: entry.params || [],
        headers: (entry.headers || []).map(([key, value]) => ({
          key,
          value,
          enabled: true,
        })),
        formRows: entry.formRows || [],
        bodyType: entry.bodyType || (entry.body ? "raw" : "none"),
        body: entry.body
          ? new TextDecoder().decode(new Uint8Array(entry.body))
          : "",
        bodyRawFormat: entry.bodyRawFormat,
      });
      // 历史只存请求，响应需重新发送
      this.response = null;
    },

    _buildUrl() {
      let url = this.url;
      const enabledParams = this.params.filter((p) => p.enabled && p.key);
      if (enabledParams.length > 0) {
        const qs = enabledParams
          .map(
            (p) =>
              `${encodeURIComponent(p.key)}=${encodeURIComponent(p.value)}`
          )
          .join("&");
        url += (url.includes("?") ? "&" : "?") + qs;
      }
      return url;
    },

    _buildHeaders() {
      const headers = this.headers
        .filter(
          (h) =>
            h.enabled &&
            h.key &&
            !SKIPPED_REQUEST_HEADERS.has(h.key.trim().toLowerCase())
        )
        .map((h) => [h.key, h.value]);

      if (this.bodyType === "raw" && this.bodyRawFormat !== "Text") {
        const has = headers.some(
          ([k]) => k.toLowerCase() === "content-type"
        );
        if (!has) {
          const types = {
            JSON: "application/json",
            XML: "application/xml",
            HTML: "text/html",
          };
          if (types[this.bodyRawFormat]) {
            headers.push(["Content-Type", types[this.bodyRawFormat]]);
          }
        }
      } else if (this.bodyType === "x-www-form-urlencoded") {
        const has = headers.some(
          ([k]) => k.toLowerCase() === "content-type"
        );
        if (!has) {
          headers.push(["Content-Type", "application/x-www-form-urlencoded"]);
        }
      }

      return headers;
    },

    _buildBodyBytes() {
      if (this.bodyType === "none") return null;

      if (this.bodyType === "x-www-form-urlencoded") {
        const qs = this.formRows
          .filter((r) => r.enabled && r.key)
          .map(
            (r) =>
              `${encodeURIComponent(r.key)}=${encodeURIComponent(r.value)}`
          )
          .join("&");
        return qs ? [...new TextEncoder().encode(qs)] : null;
      }

      if (this.bodyType === "raw" && this.body) {
        return [...new TextEncoder().encode(this.body)];
      }

      return null;
    },
  },
});
