# Request Sender（发送器）Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a Postman-like HTTP request sender page to the Flowly desktop app, with request building, response viewing, and persistent history.

**Architecture:** New Rust module `sender.rs` handles HTTP request execution (reusing `gen_client` from `mitm_core`). New module `history.rs` manages persistent history (JSON file). Frontend adds a `/sender` route with left-right split layout: `HistoryPanel` (left) + `RequestBuilder`/`ResponseViewer` (right), coordinated by a Pinia `sender` store.

**Tech Stack:** Rust/hyper (HTTP client), Vue 3 + Element Plus + Pinia, CodeMirror (body editor), vue-codemirror

## Global Constraints

- Rust edition 2021, serde + serde_json for serialization
- Frontend: Vue 3 Composition API `<script setup>`, no TypeScript, Element Plus 2.14, Pinia options-style stores
- Follow existing naming: camelCase for Tauri command serde fields, `--gm-*` CSS variables for theme
- History persisted to `data_dir/history.json`, max 200 entries
- `send_request` command supports optional proxy routing through Flowly's own listen address
- cURL parsing done in frontend JS (no backend dependency)

---

## File Structure

### New Files

| File | Responsibility |
|------|----------------|
| `app/src-tauri/src/sender.rs` | `send_request` Tauri command: build hyper Request, choose client (direct/proxy), return response |
| `app/src-tauri/src/history.rs` | History CRUD commands + JSON file persistence |
| `app/src/pages/Sender.vue` | Top-level page: left-right split layout |
| `app/src/components/HistoryPanel.vue` | Left sidebar: search, date-grouped list, clear button |
| `app/src/components/RequestBuilder.vue` | URL bar + Params/Headers/Body/Auth tabs |
| `app/src/components/ResponseViewer.vue` | Status bar + Body/Headers/Preview tabs |
| `app/src/stores/sender.js` | Pinia store: current request state, response, history |
| `app/src/utils/curl.js` | Parse cURL command → request object; generate cURL from request object |

### Modified Files

| File | Change |
|------|--------|
| `app/src-tauri/src/lib.rs` | Add `mod sender; mod history;` + register new commands in `invoke_handler` + add `history_path` to `AppState` |
| `app/src-tauri/src/state.rs` | Add `history_path: PathBuf` to `AppState` |
| `app/src/router.js` | Add `/sender` route |
| `app/src/App.vue` | Add sidebar menu item + page description |
| `app/package.json` | Add `@codemirror/lang-xml` dependency |

---

### Task 1: Backend — History Module

**Files:**
- Create: `app/src-tauri/src/history.rs`
- Modify: `app/src-tauri/src/state.rs:17-35`
- Modify: `app/src-tauri/src/lib.rs:1-10,76-87,137-156`

**Interfaces:**
- Produces: `history_list() -> Vec<HistoryEntry>`, `history_save(entry: HistoryEntry) -> HistoryEntry`, `history_clear()`, `history_delete(id: u64)`
- Consumes: `AppState.history_path`

- [ ] **Step 1: Create history.rs with data types and file I/O**

```rust
// app/src-tauri/src/history.rs

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Mutex;

const MAX_HISTORY: usize = 200;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: u64,
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<Vec<u8>>,
    pub through_proxy: bool,
    pub status: u16,
    pub status_text: String,
    pub response_headers: Vec<(String, String)>,
    pub response_body: Option<Vec<u8>>,
    pub duration_ms: u64,
    pub timestamp: i64,
}

pub struct HistoryStore {
    entries: Mutex<Vec<HistoryEntry>>,
    next_id: Mutex<u64>,
}

impl HistoryStore {
    pub fn new(path: &Path) -> Self {
        let entries = load_history(path);
        let next_id = entries.iter().map(|e| e.id).max().unwrap_or(0) + 1;
        Self {
            entries: Mutex::new(entries),
            next_id: Mutex::new(next_id),
        }
    }

    pub fn list(&self) -> Vec<HistoryEntry> {
        self.entries.lock().unwrap().clone()
    }

    pub fn save(&self, mut entry: HistoryEntry) -> HistoryEntry {
        let mut entries = self.entries.lock().unwrap();
        let mut next_id = self.next_id.lock().unwrap();
        entry.id = *next_id;
        *next_id += 1;
        entries.insert(0, entry.clone());
        if entries.len() > MAX_HISTORY {
            entries.truncate(MAX_HISTORY);
        }
        entry
    }

    pub fn clear(&self) {
        self.entries.lock().unwrap().clear();
    }

    pub fn delete(&self, id: u64) {
        self.entries.lock().unwrap().retain(|e| e.id != id);
    }

    pub fn persist(&self, path: &Path) -> Result<(), String> {
        let entries = self.entries.lock().unwrap();
        let json = serde_json::to_string_pretty(&*entries)
            .map_err(|e| format!("序列化历史记录失败: {e}"))?;
        std::fs::write(path, json).map_err(|e| format!("写历史记录失败: {e}"))?;
        Ok(())
    }
}

fn load_history(path: &Path) -> Vec<HistoryEntry> {
    let Ok(content) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    serde_json::from_str(&content).unwrap_or_default()
}
```

- [ ] **Step 2: Add history_path to AppState in state.rs**

Add to `AppState` struct after `config_path`:

```rust
    /// 历史记录持久化文件（history.json）。
    pub history_path: PathBuf,
```

- [ ] **Step 3: Register history module and commands in lib.rs**

Add `pub mod history;` to the module declarations at the top.

Add `history_path` initialization in `setup()`, after `config_path`:

```rust
    let history_path = data_dir.join("history.json");
```

Add to `app.manage(AppState { ... })`:

```rust
        history_path,
```

Add four Tauri commands in history.rs:

```rust
#[tauri::command]
pub fn history_list(state: tauri::State<'_, crate::state::AppState>) -> Result<Vec<HistoryEntry>, String> {
    Ok(state.history.list())
}

#[tauri::command]
pub fn history_save(
    state: tauri::State<'_, crate::state::AppState>,
    entry: HistoryEntry,
) -> Result<HistoryEntry, String> {
    let saved = state.history.save(entry);
    state.history.persist(&state.history_path)?;
    Ok(saved)
}

#[tauri::command]
pub fn history_clear(state: tauri::State<'_, crate::state::AppState>) -> Result<(), String> {
    state.history.clear();
    state.history.persist(&state.history_path)?;
    Ok(())
}

#[tauri::command]
pub fn history_delete(
    state: tauri::State<'_, crate::state::AppState>,
    id: u64,
) -> Result<(), String> {
    state.history.delete(id);
    state.history.persist(&state.history_path)?;
    Ok(())
}
```

Add `history: HistoryStore` field to `AppState`:

```rust
    pub history: crate::history::HistoryStore,
```

Initialize in `setup()`:

```rust
    let history = crate::history::HistoryStore::new(&history_path);
```

And in `app.manage(AppState { ... })`:

```rust
        history,
```

Register commands in `invoke_handler`:

```rust
            history::history_list,
            history::history_save,
            history::history_clear,
            history::history_delete,
```

- [ ] **Step 4: Build to verify compilation**

Run: `cd app/src-tauri && cargo check`
Expected: Compiles without errors.

- [ ] **Step 5: Commit**

```bash
git add app/src-tauri/src/history.rs app/src-tauri/src/state.rs app/src-tauri/src/lib.rs
git commit -m "feat(sender): add history backend module with CRUD commands"
```

---

### Task 2: Backend — Send Request Command

**Files:**
- Create: `app/src-tauri/src/sender.rs`

**Interfaces:**
- Consumes: `AppState.config`, `AppState.proxy` (for listen_addr)
- Produces: `send_request() -> Result<SendResponse, String>`

- [ ] **Step 1: Create sender.rs with send_request command**

```rust
// app/src-tauri/src/sender.rs

use std::time::Instant;

use mitm_core::{
    hyper::{Body, Request, Uri, body::to_bytes, header},
    http_client::{HttpClient, gen_client},
};
use serde::{Deserialize, Serialize};

use crate::state::AppState;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SendResponse {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub duration_ms: u64,
}

#[tauri::command]
pub async fn send_request(
    state: tauri::State<'_, AppState>,
    method: String,
    url: String,
    headers: Vec<(String, String)>,
    body: Option<Vec<u8>>,
    through_proxy: bool,
) -> Result<SendResponse, String> {
    let uri: Uri = url.parse().map_err(|e| format!("URL 无效: {e}"))?;

    let mut builder = Request::builder().method(method.as_str()).uri(&uri);
    for (name, value) in &headers {
        builder = builder.header(name.as_str(), value.as_str());
    }
    let request = builder
        .body(Body::from(body.unwrap_or_default()))
        .map_err(|e| format!("构造请求失败: {e}"))?;

    let client = if through_proxy {
        let proxy_addr = {
            let guard = state.proxy.lock().unwrap();
            guard
                .as_ref()
                .map(|h| h.listen_addr.clone())
                .ok_or_else(|| "代理未启动，无法经过代理发送。请先启动代理或取消勾选。".to_string())?
        };
        let proxy_uri: Uri = format!("http://{proxy_addr}")
            .parse()
            .map_err(|e| format!("代理地址无效: {e}"))?;
        let proxy = flowly::hyper_proxy::Proxy::new(flowly::hyper_proxy::Intercept::All, proxy_uri);
        gen_client(Some(proxy)).map_err(|e| format!("创建代理客户端失败: {e}"))?
    } else {
        gen_client(None).map_err(|e| format!("创建客户端失败: {e}"))?
    };

    let start = Instant::now();
    let response = match client {
        HttpClient::Https(c) => c.request(request).await.map_err(|e| format!("请求失败: {e}"))?,
        HttpClient::Proxy(c) => c.request(request).await.map_err(|e| format!("请求失败: {e}"))?,
    };
    let duration_ms = start.elapsed().as_millis() as u64;

    let (parts, body) = response.into_parts();
    let resp_headers: Vec<(String, String)> = parts
        .headers
        .iter()
        .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or_default().to_string()))
        .collect();
    let bytes = to_bytes(body)
        .await
        .map_err(|e| format!("读取响应失败: {e}"))?;

    let status_text = parts.status.canonical_reason().unwrap_or("Unknown").to_string();

    Ok(SendResponse {
        status: parts.status.as_u16(),
        status_text,
        headers: resp_headers,
        body: bytes.to_vec(),
        duration_ms,
    })
}
```

- [ ] **Step 2: Register sender module and command in lib.rs**

Add `pub mod sender;` to module declarations.

Register in `invoke_handler`:

```rust
            sender::send_request,
```

- [ ] **Step 3: Build to verify compilation**

Run: `cd app/src-tauri && cargo check`
Expected: Compiles without errors.

- [ ] **Step 4: Commit**

```bash
git add app/src-tauri/src/sender.rs app/src-tauri/src/lib.rs
git commit -m "feat(sender): add send_request command with proxy routing"
```

---

### Task 3: Frontend — cURL Parser Utility

**Files:**
- Create: `app/src/utils/curl.js`

**Interfaces:**
- Produces: `parseCurl(text) -> { method, url, headers, body, bodyType } | null`
- Produces: `toCurl(request) -> string`

- [ ] **Step 1: Create curl.js with parse and generate functions**

```js
// app/src/utils/curl.js

/**
 * 解析 cURL 命令字符串为请求对象。
 * @param {string} text - cURL 命令文本
 * @returns {{ method: string, url: string, headers: Array<{key:string, value:string, enabled:boolean}>, bodyType: string, body: string } | null}
 */
export function parseCurl(text) {
  if (!text || !text.trim().startsWith("curl")) return null;

  try {
    const args = tokenize(text);
    let method = null;
    let url = "";
    const headers = [];
    let data = null;
    let dataRaw = null;

    for (let i = 0; i < args.length; i++) {
      const arg = args[i];
      if (arg === "curl") continue;

      if (arg === "-X" || arg === "--request") {
        method = args[++i]?.toUpperCase();
      } else if (arg === "-H" || arg === "--header") {
        const headerStr = args[++i] || "";
        const colonIdx = headerStr.indexOf(":");
        if (colonIdx > 0) {
          headers.push({
            key: headerStr.slice(0, colonIdx).trim(),
            value: headerStr.slice(colonIdx + 1).trim(),
            enabled: true,
          });
        }
      } else if (arg === "-d" || arg === "--data" || arg === "--data-raw") {
        data = args[++i] || "";
      } else if (arg === "--data-urlencode") {
        data = args[++i] || "";
      } else if (arg.startsWith("-") || arg.startsWith("--")) {
        // 跳过未知 flag 及其值
        if (i + 1 < args.length && !args[i + 1].startsWith("-")) {
          i++;
        }
      } else if (!url) {
        url = arg;
      }
    }

    if (!url) return null;
    if (!url.startsWith("http://") && !url.startsWith("https://")) {
      url = "http://" + url;
    }

    let bodyType = "none";
    let body = "";
    if (data != null) {
      body = data;
      bodyType = "raw";
      const ctHeader = headers.find((h) => h.key.toLowerCase() === "content-type");
      if (!ctHeader) {
        if (body.startsWith("{") || body.startsWith("[")) {
          headers.push({ key: "Content-Type", value: "application/json", enabled: true });
        } else if (body.includes("=")) {
          headers.push({
            key: "Content-Type",
            value: "application/x-www-form-urlencoded",
            enabled: true,
          });
        }
      }
    }

    if (!method) {
      method = data != null ? "POST" : "GET";
    }

    return { method, url, headers, bodyType, body };
  } catch {
    return null;
  }
}

/**
 * 将请求对象转为 cURL 命令字符串。
 */
export function toCurl(request) {
  const parts = [`curl`];

  parts.push(`-X ${request.method}`);
  parts.push(`'${request.url}'`);

  for (const h of request.headers || []) {
    if (h.enabled !== false) {
      parts.push(`-H '${h.key}: ${h.value}'`);
    }
  }

  if (request.body && request.bodyType !== "none") {
    parts.push(`-d '${request.body.replace(/'/g, "'\\''")}'`);
  }

  return parts.join(" \\\n  ");
}

/** 简易 shell token 解析：处理单引号和双引号包裹的参数。 */
function tokenize(text) {
  const tokens = [];
  let current = "";
  let inQuote = null;

  for (let i = 0; i < text.length; i++) {
    const ch = text[i];

    if (inQuote) {
      if (ch === inQuote) {
        inQuote = null;
      } else {
        current += ch;
      }
      continue;
    }

    if (ch === "'" || ch === '"') {
      inQuote = ch;
      continue;
    }

    if (ch === "\\" && i + 1 < text.length) {
      current += text[++i];
      continue;
    }

    if (/\s/.test(ch)) {
      if (current) {
        tokens.push(current);
        current = "";
      }
      continue;
    }

    current += ch;
  }

  if (current) tokens.push(current);
  return tokens;
}
```

- [ ] **Step 2: Commit**

```bash
git add app/src/utils/curl.js
git commit -m "feat(sender): add cURL parse and generate utility"
```

---

### Task 4: Frontend — Sender Pinia Store

**Files:**
- Create: `app/src/stores/sender.js`

**Interfaces:**
- Consumes: Tauri invoke commands `send_request`, `history_list`, `history_save`, `history_clear`, `history_delete`
- Produces: reactive state for `Sender.vue`, `RequestBuilder.vue`, `ResponseViewer.vue`, `HistoryPanel.vue`

- [ ] **Step 1: Create sender.js store**

```js
// app/src/stores/sender.js

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
    response: null, // { status, statusText, headers, body, durationMs }
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
      this.headers = req.headers || [{ key: "Accept", value: "*/*", enabled: true }];
      this.bodyType = req.bodyType || "none";
      this.body = req.body || "";
      this.bodyRawFormat = req.bodyRawFormat || "Text";
      this.throughProxy = req.throughProxy ?? true;
    },

    setResponse(resp) {
      this.response = resp;
    },

    async send() {
      this.sending = true;
      this.error = null;
      this.response = null;

      const finalUrl = this._buildUrl();
      const reqHeaders = this._buildHeaders();

      try {
        const resp = await invoke("send_request", {
          method: this.method,
          url: finalUrl,
          headers: reqHeaders,
          body: this.bodyType !== "none" && this.body ? [...new TextEncoder().encode(this.body)] : null,
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
            body: this.bodyType !== "none" ? [...new TextEncoder().encode(this.body)] : null,
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
        headers: (entry.headers || []).map(([key, value]) => ({ key, value, enabled: true })),
        bodyType: entry.body ? "raw" : "none",
        body: entry.body ? new TextDecoder().decode(new Uint8Array(entry.body)) : "",
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
          .map((p) => `${encodeURIComponent(p.key)}=${encodeURIComponent(p.value)}`)
          .join("&");
        url += (url.includes("?") ? "&" : "?") + qs;
      }
      return url;
    },

    _buildHeaders() {
      const headers = this.headers
        .filter((h) => h.enabled && h.key)
        .map((h) => [h.key, h.value]);

      // 根据 bodyType 自动添加 Content-Type
      if (this.bodyType === "raw" && this.bodyRawFormat !== "Text") {
        const has = headers.some(([k]) => k.toLowerCase() === "content-type");
        if (!has) {
          const types = { JSON: "application/json", XML: "application/xml", HTML: "text/html" };
          if (types[this.bodyRawFormat]) {
            headers.push(["Content-Type", types[this.bodyRawFormat]]);
          }
        }
      } else if (this.bodyType === "x-www-form-urlencoded") {
        const has = headers.some(([k]) => k.toLowerCase() === "content-type");
        if (!has) {
          headers.push(["Content-Type", "application/x-www-form-urlencoded"]);
        }
      }

      return headers;
    },
  },
});
```

- [ ] **Step 2: Commit**

```bash
git add app/src/stores/sender.js
git commit -m "feat(sender): add Pinia store for request state and history"
```

---

### Task 5: Frontend — HistoryPanel Component

**Files:**
- Create: `app/src/components/HistoryPanel.vue`

**Interfaces:**
- Consumes: `useSenderStore().history`, `useSenderStore().loadFromHistory()`, `useSenderStore().deleteHistory()`, `useSenderStore().clearHistory()`
- Produces: left sidebar panel with search, date-grouped list, clear button

- [ ] **Step 1: Create HistoryPanel.vue**

```vue
<!-- app/src/components/HistoryPanel.vue -->
<template>
  <div class="history-panel">
    <div class="history-header">
      <span class="history-title">历史记录</span>
      <el-button text size="small" @click="confirmClear" :disabled="!store.history.length">
        清空
      </el-button>
    </div>
    <el-input
      v-model="searchText"
      placeholder="搜索 URL..."
      size="small"
      clearable
      class="history-search"
    />
    <div class="history-list" @contextmenu.prevent>
      <template v-for="group in filteredGroups" :key="group.label">
        <div class="history-group-label">{{ group.label }}</div>
        <div
          v-for="item in group.items"
          :key="item.id"
          class="history-item"
          :class="{ active: selectedId === item.id }"
          @click="selectEntry(item)"
          @contextmenu.prevent="showContextMenu($event, item)"
        >
          <span class="method-tag" :class="'method-' + item.method.toLowerCase()">
            {{ item.method }}
          </span>
          <span class="item-url" :title="item.url">{{ extractPath(item.url) }}</span>
          <div class="item-meta">
            <span class="item-status" :class="statusClass(item.status)">{{ item.status }}</span>
            <span class="item-time">{{ item.durationMs }}ms</span>
          </div>
        </div>
      </template>
      <div v-if="filteredGroups.length === 0" class="history-empty">无记录</div>
    </div>

    <!-- 右键菜单 -->
    <div
      v-if="ctxMenu.visible"
      class="history-context-menu"
      :style="{ left: ctxMenu.x + 'px', top: ctxMenu.y + 'px' }"
    >
      <div class="ctx-item" @click="copyAsCurl">复制为 cURL</div>
      <div class="ctx-item danger" @click="deleteEntry">删除</div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onBeforeUnmount } from "vue";
import { ElMessageBox, ElMessage } from "element-plus";
import { useSenderStore } from "../stores/sender";
import { toCurl } from "../utils/curl";

const store = useSenderStore();
const searchText = ref("");
const selectedId = ref(null);
const ctxMenu = ref({ visible: false, x: 0, y: 0, item: null });

onMounted(() => {
  store.loadHistory();
  document.addEventListener("click", hideContextMenu);
});

onBeforeUnmount(() => {
  document.removeEventListener("click", hideContextMenu);
});

function hideContextMenu() {
  ctxMenu.value.visible = false;
}

const filteredGroups = computed(() => {
  const items = store.history.filter((h) =>
    searchText.value ? h.url.toLowerCase().includes(searchText.value.toLowerCase()) : true
  );
  const now = new Date();
  const todayStart = new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime();
  const yesterdayStart = todayStart - 86400000;

  const groups = [];
  const today = [];
  const yesterday = [];
  const earlier = [];

  for (const item of items) {
    if (item.timestamp >= todayStart) today.push(item);
    else if (item.timestamp >= yesterdayStart) yesterday.push(item);
    else earlier.push(item);
  }

  if (today.length) groups.push({ label: "今天", items: today });
  if (yesterday.length) groups.push({ label: "昨天", items: yesterday });
  if (earlier.length) groups.push({ label: "更早", items: earlier });
  return groups;
});

function selectEntry(item) {
  selectedId.value = item.id;
  store.loadFromHistory(item);
}

function extractPath(url) {
  try {
    const u = new URL(url);
    return u.pathname === "/" ? u.host : u.host + u.pathname;
  } catch {
    return url.slice(0, 40);
  }
}

function statusClass(status) {
  if (status >= 200 && status < 300) return "status-ok";
  if (status >= 300 && status < 400) return "status-redirect";
  if (status >= 400 && status < 500) return "status-warn";
  return "status-err";
}

function showContextMenu(event, item) {
  ctxMenu.value = { visible: true, x: event.clientX, y: event.clientY, item };
}

async function copyAsCurl() {
  const item = ctxMenu.value.item;
  if (!item) return;
  const curl = toCurl({
    method: item.method,
    url: item.url,
    headers: (item.headers || []).map(([key, value]) => ({ key, value, enabled: true })),
    body: item.body ? new TextDecoder().decode(new Uint8Array(item.body)) : "",
    bodyType: item.body ? "raw" : "none",
  });
  try {
    await navigator.clipboard.writeText(curl);
    ElMessage.success("已复制为 cURL");
  } catch (e) {
    ElMessage.error("复制失败: " + e);
  }
  ctxMenu.value.visible = false;
}

async function deleteEntry() {
  const item = ctxMenu.value.item;
  if (!item) return;
  await store.deleteHistory(item.id);
  if (selectedId.value === item.id) selectedId.value = null;
  ctxMenu.value.visible = false;
}

async function confirmClear() {
  try {
    await ElMessageBox.confirm("确定清空所有历史记录？", "清空历史", { type: "warning" });
    await store.clearHistory();
    selectedId.value = null;
  } catch {
    // 用户取消
  }
}
</script>

<style scoped>
.history-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  border-right: 1px solid var(--gm-line);
  background: rgba(15, 27, 45, 0.78);
}
.history-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 12px 8px;
}
.history-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--gm-text);
}
.history-search {
  padding: 0 12px 8px;
}
.history-list {
  flex: 1;
  overflow-y: auto;
  padding: 0 8px;
}
.history-group-label {
  font-size: 11px;
  color: var(--gm-subtle);
  padding: 8px 4px 4px;
  border-bottom: 1px solid var(--gm-line);
  margin-bottom: 4px;
}
.history-item {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
  padding: 6px 8px;
  border-radius: 4px;
  cursor: pointer;
  transition: background 0.15s;
}
.history-item:hover {
  background: rgba(56, 189, 248, 0.08);
}
.history-item.active {
  background: rgba(56, 189, 248, 0.15);
}
.method-tag {
  font-size: 10px;
  font-weight: 700;
  padding: 1px 5px;
  border-radius: 3px;
  flex-shrink: 0;
}
.method-get { background: rgba(34, 197, 94, 0.15); color: #22c55e; }
.method-post { background: rgba(56, 189, 248, 0.15); color: #38bdf8; }
.method-put { background: rgba(245, 158, 11, 0.15); color: #f59e0b; }
.method-delete { background: rgba(248, 113, 113, 0.15); color: #f87171; }
.method-patch { background: rgba(168, 85, 247, 0.15); color: #a855f7; }
.method-options, .method-head { background: rgba(148, 163, 184, 0.15); color: #94a3b8; }
.item-url {
  font-size: 12px;
  color: var(--gm-text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
  min-width: 0;
}
.item-meta {
  display: flex;
  gap: 6px;
  font-size: 10px;
  width: 100%;
  padding-left: 36px;
}
.item-status { font-weight: 600; }
.status-ok { color: #22c55e; }
.status-redirect { color: #38bdf8; }
.status-warn { color: #f59e0b; }
.status-err { color: #f87171; }
.item-time { color: var(--gm-subtle); }
.history-empty {
  text-align: center;
  color: var(--gm-subtle);
  padding: 24px;
  font-size: 12px;
}
.history-context-menu {
  position: fixed;
  z-index: 3000;
  background: #1a2740;
  border: 1px solid var(--gm-line);
  border-radius: 4px;
  padding: 4px 0;
  min-width: 120px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
}
.ctx-item {
  padding: 6px 12px;
  font-size: 12px;
  color: var(--gm-text);
  cursor: pointer;
}
.ctx-item:hover {
  background: rgba(56, 189, 248, 0.1);
}
.ctx-item.danger {
  color: #f87171;
}
</style>
```

- [ ] **Step 2: Commit**

```bash
git add app/src/components/HistoryPanel.vue
git commit -m "feat(sender): add HistoryPanel component with search and context menu"
```

---

### Task 6: Frontend — RequestBuilder Component

**Files:**
- Create: `app/src/components/RequestBuilder.vue`
- Install: `@codemirror/lang-xml`

**Interfaces:**
- Consumes: `useSenderStore()` — method, url, params, headers, bodyType, body, bodyRawFormat, throughProxy, sending
- Produces: emits `send` event, handles cURL paste on URL input

- [ ] **Step 1: Install @codemirror/lang-xml**

Run: `cd app && npm install @codemirror/lang-xml`

- [ ] **Step 2: Create RequestBuilder.vue**

```vue
<!-- app/src/components/RequestBuilder.vue -->
<template>
  <div class="request-builder">
    <!-- URL 行 -->
    <div class="url-bar">
      <el-select v-model="store.method" size="small" style="width: 100px" class="method-select">
        <el-option v-for="m in methods" :key="m" :value="m" :label="m" />
      </el-select>
      <el-input
        v-model="store.url"
        placeholder="输入 URL 或粘贴 cURL 命令..."
        size="small"
        class="url-input"
        @paste="onPaste"
      />
      <el-button
        type="primary"
        size="small"
        :loading="store.sending"
        @click="$emit('send')"
      >
        Send
      </el-button>
    </div>
    <div class="url-options">
      <el-checkbox v-model="store.throughProxy" size="small">经过代理</el-checkbox>
    </div>

    <!-- Tab 面板 -->
    <el-tabs v-model="activeTab" class="request-tabs">
      <el-tab-pane label="Params" name="params">
        <KeyValueTable v-model="store.params" add-label="+ 添加参数" />
      </el-tab-pane>
      <el-tab-pane label="Headers" name="headers">
        <KeyValueTable v-model="store.headers" add-label="+ 添加请求头" />
      </el-tab-pane>
      <el-tab-pane label="Body" name="body">
        <div class="body-type-row">
          <el-radio-group v-model="store.bodyType" size="small">
            <el-radio-button value="none">none</el-radio-button>
            <el-radio-button value="form-data">form-data</el-radio-button>
            <el-radio-button value="x-www-form-urlencoded">x-www-form-urlencoded</el-radio-button>
            <el-radio-button value="raw">raw</el-radio-button>
          </el-radio-group>
          <el-select
            v-if="store.bodyType === 'raw'"
            v-model="store.bodyRawFormat"
            size="small"
            style="width: 80px; margin-left: 8px"
          >
            <el-option value="Text" label="Text" />
            <el-option value="JSON" label="JSON" />
            <el-option value="XML" label="XML" />
            <el-option value="HTML" label="HTML" />
          </el-select>
        </div>
        <div v-if="store.bodyType === 'none'" class="body-empty">此请求没有 Body</div>
        <KeyValueTable
          v-else-if="store.bodyType === 'form-data' || store.bodyType === 'x-www-form-urlencoded'"
          v-model="formDataRows"
          add-label="+ 添加字段"
        />
        <div v-else-if="store.bodyType === 'raw'" class="body-editor">
          <codemirror
            v-model="store.body"
            :style="{ height: '200px', fontSize: '13px' }"
            :extensions="bodyExtensions"
          />
        </div>
      </el-tab-pane>
      <el-tab-pane label="Auth" name="auth">
        <div class="body-empty">即将支持</div>
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup>
import { ref, computed, h } from "vue";
import { ElMessage } from "element-plus";
import { Codemirror } from "vue-codemirror";
import { json } from "@codemirror/lang-json";
import { xml } from "@codemirror/lang-xml";
import { useSenderStore } from "../stores/sender";
import { parseCurl } from "../utils/curl";
import KeyValueTable from "./KeyValueTable.vue";

defineEmits(["send"]);

const store = useSenderStore();
const activeTab = ref("params");
const methods = ["GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS", "HEAD"];

const bodyExtensions = computed(() => {
  const exts = [];
  if (store.bodyRawFormat === "JSON") exts.push(json());
  else if (store.bodyRawFormat === "XML" || store.bodyRawFormat === "HTML") exts.push(xml());
  return exts;
});

const formDataRows = computed({
  get() {
    return store._formRows || [];
  },
  set(val) {
    store._formRows = val;
  },
});

function onPaste(event) {
  const text = event.clipboardData?.getData("text") || "";
  if (!text.trim().startsWith("curl")) return;

  const parsed = parseCurl(text);
  if (!parsed) {
    ElMessage.warning("无法解析 cURL 命令");
    return;
  }

  event.preventDefault();
  store.method = parsed.method;
  store.url = parsed.url;
  if (parsed.headers.length) store.headers = parsed.headers;
  if (parsed.body) {
    store.body = parsed.body;
    store.bodyType = parsed.bodyType;
    if (parsed.bodyType === "raw" && (parsed.body.startsWith("{") || parsed.body.startsWith("["))) {
      store.bodyRawFormat = "JSON";
    }
  }
}
</script>

<style scoped>
.request-builder {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.url-bar {
  display: flex;
  gap: 8px;
  align-items: center;
}
.url-input {
  flex: 1;
}
.method-select :deep(.el-select__wrapper) {
  background: rgba(15, 27, 45, 0.9);
  box-shadow: 0 0 0 1px var(--gm-line) inset;
}
.url-options {
  display: flex;
  align-items: center;
  gap: 12px;
}
.request-tabs :deep(.el-tabs__header) {
  margin-bottom: 8px;
}
.body-type-row {
  display: flex;
  align-items: center;
  margin-bottom: 8px;
}
.body-empty {
  color: var(--gm-subtle);
  font-size: 12px;
  padding: 24px;
  text-align: center;
}
.body-editor {
  border: 1px solid var(--gm-line);
  border-radius: 4px;
  overflow: hidden;
}
</style>
```

- [ ] **Step 3: Commit**

```bash
git add app/package.json app/package-lock.json app/src/components/RequestBuilder.vue
git commit -m "feat(sender): add RequestBuilder component with cURL paste support"
```

---

### Task 7: Frontend — KeyValueTable Sub-component

**Files:**
- Create: `app/src/components/KeyValueTable.vue`

**Interfaces:**
- Consumes: `modelValue` array of `{ key, value, enabled }`
- Produces: emits `update:modelValue`

- [ ] **Step 1: Create KeyValueTable.vue**

```vue
<!-- app/src/components/KeyValueTable.vue -->
<template>
  <div class="kv-table">
    <div v-for="(row, i) in modelValue" :key="i" class="kv-row">
      <el-checkbox v-model="row.enabled" size="small" />
      <el-input v-model="row.key" placeholder="Key" size="small" class="kv-input" />
      <el-input v-model="row.value" placeholder="Value" size="small" class="kv-input" />
      <el-button text size="small" @click="removeRow(i)" class="kv-delete">×</el-button>
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
  emit("update:modelValue", [...props.modelValue, { key: "", value: "", enabled: true }]);
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
```

- [ ] **Step 2: Commit**

```bash
git add app/src/components/KeyValueTable.vue
git commit -m "feat(sender): add KeyValueTable sub-component"
```

---

### Task 8: Frontend — ResponseViewer Component

**Files:**
- Create: `app/src/components/ResponseViewer.vue`

**Interfaces:**
- Consumes: `useSenderStore().response`, `useSenderStore().error`, `useSenderStore().sending`
- Produces: status bar + Body (Pretty/Raw) + Headers + Preview tabs

- [ ] **Step 1: Create ResponseViewer.vue**

```vue
<!-- app/src/components/ResponseViewer.vue -->
<template>
  <div class="response-viewer">
    <!-- 状态栏 -->
    <div class="response-status-bar">
      <span class="response-label">Response</span>
      <template v-if="store.response">
        <el-tag :type="statusTagType" size="small" effect="dark" class="status-tag">
          {{ store.response.status }} {{ store.response.statusText }}
        </el-tag>
        <span class="stat">{{ store.response.durationMs }}ms</span>
        <span class="stat">{{ formatSize(store.response.size) }}</span>
      </template>
      <span v-else-if="store.error" class="error-text">{{ store.error }}</span>
      <span v-else-if="store.sending" class="stat">
        <el-icon class="spinning"><Loading /></el-icon> 请求中...
      </span>
      <span v-else class="empty-hint">点击 Send 发送请求</span>
    </div>

    <!-- Tab 面板 -->
    <el-tabs v-if="store.response" v-model="activeTab" class="response-tabs">
      <el-tab-pane label="Body" name="body">
        <div class="body-sub-tabs">
          <el-radio-group v-model="bodyView" size="small">
            <el-radio-button value="pretty">Pretty</el-radio-button>
            <el-radio-button value="raw">Raw</el-radio-button>
          </el-radio-group>
        </div>
        <div v-if="bodyView === 'pretty'" class="body-content">
          <codemirror
            :model-value="prettyBody"
            :style="{ height: '100%', fontSize: '13px' }"
            :extensions="prettyExtensions"
            :readonly="true"
          />
        </div>
        <pre v-else class="raw-body">{{ store.response.body }}</pre>
      </el-tab-pane>
      <el-tab-pane label="Headers" name="headers">
        <table class="headers-table">
          <tbody>
            <tr v-for="(h, i) in store.response.headers" :key="i">
              <td class="header-name">{{ h[0] }}</td>
              <td class="header-value">{{ h[1] }}</td>
            </tr>
          </tbody>
        </table>
      </el-tab-pane>
      <el-tab-pane label="Preview" name="preview">
        <div v-if="previewType === 'html'" class="preview-frame">
          <iframe :srcdoc="store.response.body" sandbox="" class="preview-iframe" />
        </div>
        <div v-else-if="previewType === 'image'" class="preview-image">
          <img :src="imageDataUrl" alt="preview" />
        </div>
        <div v-else class="body-empty">此内容类型不支持预览</div>
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup>
import { ref, computed } from "vue";
import { Loading } from "@element-plus/icons-vue";
import { Codemirror } from "vue-codemirror";
import { json } from "@codemirror/lang-json";
import { xml } from "@codemirror/lang-xml";
import { useSenderStore } from "../stores/sender";

const store = useSenderStore();
const activeTab = ref("body");
const bodyView = ref("pretty");

const statusTagType = computed(() => {
  const s = store.response?.status;
  if (!s) return "info";
  if (s < 300) return "success";
  if (s < 400) return "";
  if (s < 500) return "warning";
  return "danger";
});

const contentType = computed(() => {
  const h = store.response?.headers?.find(([k]) => k.toLowerCase() === "content-type");
  return h ? h[1].toLowerCase() : "";
});

const prettyExtensions = computed(() => {
  const ct = contentType.value;
  if (ct.includes("json")) return [json()];
  if (ct.includes("xml") || ct.includes("html")) return [xml()];
  return [];
});

const prettyBody = computed(() => {
  const body = store.response?.body || "";
  if (contentType.value.includes("json")) {
    try {
      return JSON.stringify(JSON.parse(body), null, 2);
    } catch {
      return body;
    }
  }
  return body;
});

const previewType = computed(() => {
  const ct = contentType.value;
  if (ct.includes("html")) return "html";
  if (ct.startsWith("image/")) return "image";
  return "none";
});

const imageDataUrl = computed(() => {
  if (!store.response?.body) return "";
  const ct = contentType.value;
  const bytes = new TextEncoder().encode(store.response.body);
  const blob = new Blob([bytes], { type: ct });
  return URL.createObjectURL(blob);
});

function formatSize(bytes) {
  if (bytes == null) return "";
  if (bytes < 1024) return bytes + " B";
  return (bytes / 1024).toFixed(1) + " KB";
}
</script>

<style scoped>
.response-viewer {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
}
.response-status-bar {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 0;
  border-bottom: 1px solid var(--gm-line);
  margin-bottom: 8px;
}
.response-label {
  font-size: 13px;
  font-weight: 600;
  color: var(--gm-text);
}
.stat {
  font-size: 12px;
  color: var(--gm-subtle);
}
.error-text {
  font-size: 12px;
  color: #f87171;
}
.empty-hint {
  font-size: 12px;
  color: var(--gm-subtle);
}
.spinning {
  animation: spin 1s linear infinite;
}
@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
.response-tabs {
  flex: 1;
  min-height: 0;
}
.body-sub-tabs {
  margin-bottom: 8px;
}
.body-content {
  border: 1px solid var(--gm-line);
  border-radius: 4px;
  overflow: hidden;
  height: calc(100% - 40px);
}
.raw-body {
  font-family: "JetBrains Mono", "Fira Code", monospace;
  font-size: 13px;
  color: var(--gm-text);
  white-space: pre-wrap;
  word-break: break-all;
  padding: 12px;
  margin: 0;
  max-height: 400px;
  overflow-y: auto;
}
.headers-table {
  width: 100%;
  font-size: 12px;
  border-collapse: collapse;
}
.headers-table td {
  padding: 4px 8px;
  border-bottom: 1px solid var(--gm-line);
}
.header-name {
  color: #38bdf8;
  font-weight: 600;
  width: 200px;
  white-space: nowrap;
}
.header-value {
  color: var(--gm-text);
  word-break: break-all;
}
.body-empty {
  color: var(--gm-subtle);
  font-size: 12px;
  padding: 24px;
  text-align: center;
}
.preview-frame {
  border: 1px solid var(--gm-line);
  border-radius: 4px;
  overflow: hidden;
  height: 300px;
}
.preview-iframe {
  width: 100%;
  height: 100%;
  border: none;
  background: #fff;
}
.preview-image img {
  max-width: 100%;
  max-height: 300px;
}
</style>
```

- [ ] **Step 2: Commit**

```bash
git add app/src/components/ResponseViewer.vue
git commit -m "feat(sender): add ResponseViewer component with Pretty/Raw/Headers/Preview"
```

---

### Task 9: Frontend — Sender Page + Route + Menu

**Files:**
- Create: `app/src/pages/Sender.vue`
- Modify: `app/src/router.js`
- Modify: `app/src/App.vue`

**Interfaces:**
- Consumes: `RequestBuilder`, `ResponseViewer`, `HistoryPanel`, `useSenderStore`

- [ ] **Step 1: Create Sender.vue**

```vue
<!-- app/src/pages/Sender.vue -->
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
```

- [ ] **Step 2: Add route in router.js**

Add import at the top:

```js
import Sender from "./pages/Sender.vue";
```

Add route in the routes array (after `/monitor`):

```js
    { path: "/sender", component: Sender, meta: { title: "发送器" } },
```

- [ ] **Step 3: Add sidebar menu item in App.vue**

Add `Promotion` to the icons import:

```js
import { Odometer, Document, Lock, Setting, Tools, Promotion } from "@element-plus/icons-vue";
```

Add menu item in the `<el-menu>` (after monitor item):

```html
        <el-menu-item index="/sender">
          <el-icon><Promotion /></el-icon>
          <span>发送器</span>
        </el-menu-item>
```

Add to `pageDescriptions` map:

```js
  "/sender": "构造和发送 HTTP 请求，调试 API 接口。",
```

- [ ] **Step 4: Build frontend to verify**

Run: `cd app && npm run build`
Expected: Builds without errors.

- [ ] **Step 5: Commit**

```bash
git add app/src/pages/Sender.vue app/src/router.js app/src/App.vue
git commit -m "feat(sender): add Sender page with route and sidebar menu entry"
```

---

### Task 10: Full Build Verification

**Files:** None (verification only)

- [ ] **Step 1: Full Tauri build**

Run: `cd app && npm run tauri build`
Expected: Both MSI and NSIS bundles built successfully.

- [ ] **Step 2: Manual smoke test checklist**

Open the app and verify:
- [ ] Sidebar shows "发送器" menu item with icon
- [ ] Clicking navigates to `/sender` page
- [ ] Left panel shows empty history state
- [ ] Enter URL `https://httpbin.org/get`, click Send
- [ ] Response shows 200 OK with status, timing, size
- [ ] Body Pretty tab shows formatted JSON
- [ ] Headers tab shows response headers
- [ ] History panel shows the request
- [ ] Paste `curl -X POST https://httpbin.org/post -H "Content-Type: application/json" -d '{"key":"value"}'` into URL input
- [ ] Fields auto-fill: method=POST, headers include Content-Type, body contains JSON
- [ ] Toggle "经过代理" off, send again — request goes direct
- [ ] Right-click history item → "复制为 cURL" copies to clipboard
- [ ] Right-click history item → "删除" removes it
- [ ] "清空" button shows confirmation dialog

- [ ] **Step 3: Final commit (if any fixes needed)**
