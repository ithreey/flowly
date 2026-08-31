import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";

const SKIPPED_REQUEST_HEADERS = new Set([
  "connection",
  "keep-alive",
  "proxy-authenticate",
  "proxy-authorization",
  "te",
  "trailer",
  "transfer-encoding",
  "upgrade",
  "host",
  "content-length",
  "if-none-match",
  "if-modified-since",
]);

function headerValue(headers, name) {
  const target = name.toLowerCase();
  const found = (headers || []).find(
    ([key]) => String(key).toLowerCase() === target,
  );
  return found ? String(found[1] || "") : "";
}

function splitUrlParams(rawUrl) {
  if (!rawUrl) return { url: "", params: [] };

  try {
    const parsed = new URL(rawUrl);
    const params = Array.from(parsed.searchParams.entries()).map(
      ([key, value]) => ({
        key,
        value,
        enabled: true,
      }),
    );
    parsed.search = "";
    parsed.hash = "";
    return { url: parsed.toString(), params };
  } catch {
    return { url: rawUrl, params: [] };
  }
}

function parseFormRows(body) {
  if (!body) return [];

  return Array.from(new URLSearchParams(body).entries()).map(
    ([key, value]) => ({
      key,
      value,
      enabled: true,
    }),
  );
}

function inferBodyRawFormat(contentType, body) {
  const type = contentType.toLowerCase();
  const trimmed = String(body || "").trim();

  if (type.includes("json") || trimmed.startsWith("{") || trimmed.startsWith("[")) {
    return "JSON";
  }
  if (type.includes("xml") || type.includes("svg")) return "XML";
  if (type.includes("html")) return "HTML";
  return "Text";
}

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
    selectedHistoryId: null,
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
        const savedEntry = await invoke("history_save", {
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
        this.selectedHistoryId = savedEntry.id;
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
      this.selectedHistoryId = null;
    },

    async deleteHistory(id) {
      await invoke("history_delete", { id });
      this.history = this.history.filter((h) => h.id !== id);
      if (this.selectedHistoryId === id) this.selectedHistoryId = null;
    },

    loadFromHistory(entry) {
      this.selectedHistoryId = entry.id;
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

    loadFromTrafficDetail(detail) {
      const summary = detail?.summary || {};
      const reqHeaders = detail?.reqHeaders || [];
      const reqBody = detail?.reqBody || "";
      const contentType = headerValue(reqHeaders, "content-type");
      const { url, params } = splitUrlParams(summary.url || "");
      const isFormBody = contentType
        .toLowerCase()
        .includes("application/x-www-form-urlencoded");

      this.setRequest({
        method: summary.method || "GET",
        url,
        params,
        headers: reqHeaders
          .filter(
            ([key]) =>
              !SKIPPED_REQUEST_HEADERS.has(String(key).trim().toLowerCase()),
          )
          .map(([key, value]) => ({
            key,
            value,
            enabled: true,
          })),
        formRows: isFormBody ? parseFormRows(reqBody) : [],
        bodyType: reqBody ? (isFormBody ? "x-www-form-urlencoded" : "raw") : "none",
        body: isFormBody ? "" : reqBody,
        bodyRawFormat: inferBodyRawFormat(contentType, reqBody),
      });
      this.response = null;
      this.error = null;
      this.selectedHistoryId = null;
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
