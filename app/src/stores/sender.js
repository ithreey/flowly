import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";

export const useSenderStore = defineStore("sender", {
  state: () => ({
    // 当前请求
    method: "GET",
    url: "",
    params: [],
    headers: [{ key: "Accept", value: "*/*", enabled: true }],
    bodyType: "none",
    body: "",
    bodyRawFormat: "Text",
    throughProxy: true,

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
      this.headers = req.headers || [
        { key: "Accept", value: "*/*", enabled: true },
      ];
      this.bodyType = req.bodyType || "none";
      this.body = req.body || "";
      this.bodyRawFormat = req.bodyRawFormat || "Text";
      this.throughProxy = req.throughProxy ?? true;
    },

    async send() {
      this.sending = true;
      this.error = null;
      this.response = null;

      const finalUrl = this._buildUrl();
      const reqHeaders = this._buildHeaders();
      const bodyBytes =
        this.bodyType !== "none" && this.body
          ? [...new TextEncoder().encode(this.body)]
          : null;

      try {
        const resp = await invoke("send_request", {
          method: this.method,
          url: finalUrl,
          headers: reqHeaders,
          body: bodyBytes,
          throughProxy: this.throughProxy,
        });

        this.response = {
          status: resp.status,
          statusText: resp.statusText,
          headers: resp.headers,
          body: new TextDecoder().decode(new Uint8Array(resp.body)),
          durationMs: resp.durationMs,
          size: resp.body.length,
        };

        // 保存到历史
        await invoke("history_save", {
          entry: {
            id: 0,
            method: this.method,
            url: finalUrl,
            headers: reqHeaders,
            body: bodyBytes,
            throughProxy: this.throughProxy,
            status: resp.status,
            statusText: resp.statusText,
            responseHeaders: resp.headers,
            responseBody: resp.body,
            durationMs: resp.durationMs,
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
        headers: (entry.headers || []).map(([key, value]) => ({
          key,
          value,
          enabled: true,
        })),
        bodyType: entry.body ? "raw" : "none",
        body: entry.body
          ? new TextDecoder().decode(new Uint8Array(entry.body))
          : "",
        throughProxy: entry.throughProxy,
      });
      this.response = {
        status: entry.status,
        statusText: entry.statusText,
        headers: entry.responseHeaders || [],
        body: entry.responseBody
          ? new TextDecoder().decode(new Uint8Array(entry.responseBody))
          : "",
        durationMs: entry.durationMs,
        size: entry.responseBody?.length || 0,
      };
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
        .filter((h) => h.enabled && h.key)
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
  },
});
