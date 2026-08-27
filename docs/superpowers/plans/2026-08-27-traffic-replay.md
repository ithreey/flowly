# Traffic Replay Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add monitor-page request replay that directly re-sends a captured request from the Tauri backend and records the replay response as a new traffic session.

**Architecture:** Implement replay as a focused backend helper plus a Tauri command. The helper reads `TransactionDetail` from `SharedTraffic`, rebuilds a `hyper::Request`, sends it through a direct `hyper::Client`, and records the response through existing traffic storage and event broadcasting. The frontend only adds a store action and a single context-menu command.

**Tech Stack:** Rust, Tauri 2, Hyper through `mitm_core::hyper`, Tokio, Vue 3, Pinia, Element Plus.

---

## File Structure

- Modify `app/src-tauri/src/traffic.rs`: add reusable replay request structs and `SharedTraffic::record_replay_result` only if a small helper is needed to keep command code short.
- Modify `app/src-tauri/src/lib.rs`: add `traffic_replay` command and register it with `tauri::generate_handler!`.
- Modify `app/src-tauri/tests/traffic_smoke.rs`: add the failing replay integration test using the existing proxy test harness.
- Modify `app/src/stores/traffic.js`: add `replay(id)` store action that invokes `traffic_replay`.
- Modify `app/src/pages/Monitor.vue`: track the right-clicked row and add the "重放请求" context-menu item.

### Task 1: Backend Replay Test

**Files:**
- Modify: `app/src-tauri/tests/traffic_smoke.rs`
- Later modify: `app/src-tauri/src/lib.rs`

- [ ] **Step 1: Write the failing test**

Add a test helper that records request method, path, and body in the local HTTP server, then add a test named `replay_captured_post_request_records_new_session`.

```rust
#[derive(Debug, Clone)]
struct SeenRequest {
    method: String,
    path: String,
    body: String,
}

async fn spawn_recording_http_server() -> (u16, Arc<tokio::sync::Mutex<Vec<SeenRequest>>>) {
    let seen = Arc::new(tokio::sync::Mutex::new(Vec::new()));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let seen_for_task = Arc::clone(&seen);
    tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                break;
            };
            let seen_for_conn = Arc::clone(&seen_for_task);
            tokio::spawn(async move {
                let mut buf = [0u8; 8192];
                let n = stream.read(&mut buf).await.unwrap_or(0);
                let raw = String::from_utf8_lossy(&buf[..n]);
                let mut parts = raw.split("\r\n\r\n");
                let head = parts.next().unwrap_or_default();
                let body = parts.next().unwrap_or_default().to_string();
                let request_line = head.lines().next().unwrap_or_default();
                let mut request_parts = request_line.split_whitespace();
                let method = request_parts.next().unwrap_or_default().to_string();
                let path = request_parts.next().unwrap_or_default().to_string();
                seen_for_conn.lock().await.push(SeenRequest { method, path, body });
                let resp = "HTTP/1.1 200 OK\r\ncontent-type: text/plain\r\ncontent-length: 8\r\nconnection: close\r\n\r\nreplayed";
                let _ = stream.write_all(resp.as_bytes()).await;
            });
        }
    });
    (port, seen)
}

#[tokio::test]
async fn replay_captured_post_request_records_new_session() {
    let (server_port, seen) = spawn_recording_http_server().await;

    let traffic = SharedTraffic::new();
    let config = Arc::new(RwLock::new(AppConfig::default()));
    let rules: Arc<RwLock<Vec<Rule>>> = Arc::new(RwLock::new(vec![]));
    let (emit_i, emit_t) = mock_intercept_sinks();
    let handler = GuiHandler::new(
        RuleHttpHandler::new(rules),
        traffic.clone(),
        config,
        emit_i,
        emit_t,
    );
    let mitm_filter: Arc<MitmFilter<RuleHandlerCtx>> = Arc::new(MitmFilter::new(vec![]));

    let (proxy_port, shutdown_tx, proxy_task) =
        spawn_proxy(handler, Arc::clone(&mitm_filter)).await;
    let client = proxy_client(proxy_port);
    let target = format!("http://127.0.0.1:{server_port}/submit");
    let req = mitm_core::hyper::Request::builder()
        .method("POST")
        .uri(target)
        .header("content-type", "text/plain")
        .body(mitm_core::hyper::Body::from("alpha=1"))
        .unwrap();

    let res = client.request(req).await.expect("proxy request");
    assert_eq!(res.status(), 200);
    let _ = to_bytes(res.into_body()).await.unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    let first = traffic.list(100, 0).pop().expect("captured session");

    flowly_gui::traffic_replay_for_test(traffic.clone(), first.id)
        .await
        .expect("replay request");

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    let seen = seen.lock().await;
    assert_eq!(seen.len(), 2);
    assert_eq!(seen[1].method, "POST");
    assert_eq!(seen[1].path, "/submit");
    assert_eq!(seen[1].body, "alpha=1");
    drop(seen);

    let list = traffic.list(100, 0);
    assert_eq!(list.len(), 2);
    assert_eq!(list[1].method, "POST");
    assert_eq!(list[1].status, Some(200));
    assert!(list[1].url.contains("/submit"));

    let _ = shutdown_tx.send(());
    let _ = tokio::time::timeout(std::time::Duration::from_secs(2), proxy_task).await;
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --manifest-path app/src-tauri/Cargo.toml replay_captured_post_request_records_new_session -- --nocapture`

Expected: FAIL because `flowly_gui::traffic_replay_for_test` does not exist.

### Task 2: Backend Replay Implementation

**Files:**
- Modify: `app/src-tauri/src/lib.rs`
- Modify: `app/src-tauri/src/traffic.rs` only if response body capture helpers need to be reused

- [ ] **Step 1: Implement minimal backend replay**

Add a public async helper and Tauri command in `app/src-tauri/src/lib.rs`.

```rust
#[tauri::command]
async fn traffic_replay(state: tauri::State<'_, AppState>, id: u64) -> Result<u64, String> {
    replay_traffic_request(state.traffic.clone(), id).await
}

#[cfg(test)]
pub async fn traffic_replay_for_test(traffic: SharedTraffic, id: u64) -> Result<u64, String> {
    replay_traffic_request(traffic, id).await
}
```

The helper should:

```rust
async fn replay_traffic_request(traffic: SharedTraffic, id: u64) -> Result<u64, String> {
    let detail = traffic
        .get(id)
        .ok_or_else(|| "会话已过期或已删除，无法重放".to_string())?;
    let replay_id = traffic.next_id();
    let method = detail.summary.method.clone();
    let url = detail.summary.url.clone();
    let host = detail.summary.host.clone();
    let req_body = detail.req_body.clone();
    let req_size = req_body.as_ref().map(|body| body.len()).unwrap_or(0);
    let req_ct = detail
        .req_headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
        .map(|(_, value)| value.clone());
    let req_headers = sanitize_replay_headers(&detail.req_headers);

    traffic.begin_request(
        replay_id,
        method.clone(),
        url.clone(),
        host,
        req_headers.clone(),
        req_body.clone(),
        req_size,
        req_ct,
    );

    let uri: mitm_core::hyper::Uri = url
        .parse()
        .map_err(|e| format!("URL 无效，无法重放：{e}"))?;
    let mut builder = mitm_core::hyper::Request::builder().method(method.as_str()).uri(uri);
    for (name, value) in &req_headers {
        builder = builder.header(name.as_str(), value.as_str());
    }
    let request = builder
        .body(mitm_core::hyper::Body::from(req_body.unwrap_or_default()))
        .map_err(|e| format!("构造重放请求失败：{e}"))?;
    let client = mitm_core::hyper::Client::new();
    let response = client
        .request(request)
        .await
        .map_err(|e| format!("重放请求失败：{e}"))?;

    let (parts, body) = response.into_parts();
    let res_ct = parts
        .headers
        .get(mitm_core::hyper::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(String::from);
    let res_headers = parts
        .headers
        .iter()
        .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or_default().to_string()))
        .collect();
    let bytes = mitm_core::hyper::body::to_bytes(body)
        .await
        .map_err(|e| format!("读取重放响应失败：{e}"))?;
    let res_size = bytes.len();
    let res_body = String::from_utf8(bytes.to_vec()).ok();
    traffic.complete(
        replay_id,
        parts.status.as_u16(),
        res_ct,
        res_headers,
        res_body,
        res_size,
        false,
    );
    Ok(replay_id)
}
```

Add header sanitizing:

```rust
fn sanitize_replay_headers(headers: &[(String, String)]) -> Vec<(String, String)> {
    headers
        .iter()
        .filter(|(name, _)| !is_hop_by_hop_replay_header(name))
        .cloned()
        .collect()
}

fn is_hop_by_hop_replay_header(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
            | "host"
            | "content-length"
    )
}
```

Register `traffic_replay` in the `tauri::generate_handler!` list.

- [ ] **Step 2: Run the backend replay test**

Run: `cargo test --manifest-path app/src-tauri/Cargo.toml replay_captured_post_request_records_new_session -- --nocapture`

Expected: PASS.

- [ ] **Step 3: Commit backend replay**

Run:

```bash
git add app/src-tauri/src/lib.rs app/src-tauri/tests/traffic_smoke.rs
git commit -m "feat: add traffic request replay backend"
```

### Task 3: Frontend Context Menu Wiring

**Files:**
- Modify: `app/src/stores/traffic.js`
- Modify: `app/src/pages/Monitor.vue`

- [ ] **Step 1: Add the store action**

In `app/src/stores/traffic.js`, add:

```js
async replay(id) {
  return await invoke("traffic_replay", { id });
},
```

- [ ] **Step 2: Add context-menu row state**

In `app/src/pages/Monitor.vue`, add:

```js
const contextMenuRow = ref(null);
```

Update `handleContextMenu`:

```js
function handleContextMenu(row, column, event) {
  event.preventDefault();
  contextMenuRow.value = row;
  if (!selectedRows.value.some((r) => r.id === row.id)) {
    tableRef.value?.toggleRowSelection(row, true);
  }
  contextMenuX.value = event.clientX;
  contextMenuY.value = event.clientY;
  contextMenuVisible.value = true;
}
```

Update click close:

```js
const handleClick = () => {
  contextMenuVisible.value = false;
  contextMenuRow.value = null;
};
```

- [ ] **Step 3: Add the replay menu action**

Add this menu item before export:

```vue
<div class="context-menu-item" @click="replayRequest">重放请求</div>
```

Add the method:

```js
async function replayRequest() {
  const row = contextMenuRow.value;
  if (!row) return;
  contextMenuVisible.value = false;
  contextMenuRow.value = null;
  try {
    await traffic.replay(row.id);
    ElMessage.success("已重放请求");
  } catch (e) {
    ElMessage.error(`重放失败：${e}`);
  }
}
```

- [ ] **Step 4: Run frontend build**

Run: `npm run build` from `app`.

Expected: PASS.

- [ ] **Step 5: Commit frontend replay**

Run:

```bash
git add app/src/stores/traffic.js app/src/pages/Monitor.vue
git commit -m "feat: add replay action to traffic menu"
```

### Task 4: Final Verification

**Files:**
- No code changes expected.

- [ ] **Step 1: Run backend tests**

Run: `cargo test --manifest-path app/src-tauri/Cargo.toml`

Expected: PASS.

- [ ] **Step 2: Run frontend build**

Run: `npm run build` from `app`.

Expected: PASS.

- [ ] **Step 3: Inspect git state**

Run: `git status --short`

Expected: only intended documentation and feature commits are present, or a clean tree if all files were committed.
