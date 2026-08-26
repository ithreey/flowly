# HAR 导出功能实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 为流量监控页面添加会话导出功能，支持多选会话并导出为标准 HAR 文件

**Architecture:** 后端新增批量查询接口，前端通过复选框多选会话，调用批量接口获取完整事务详情，转换为 HAR 格式后通过文件保存对话框导出

**Tech Stack:** Rust (Tauri), Vue 3 (Element Plus), HAR 1.2 规范

## Global Constraints

- Rust 代码必须符合现有代码风格（使用 `serde::Serialize`，`#[serde(rename_all = "camelCase")]`）
- 前端代码使用 Vue 3 Composition API + Element Plus
- HAR 文件必须符合 HAR 1.2 规范
- 一次 IPC 调用完成批量查询，避免循环调用
- 文件名格式：`traffic_YYYY-MM-DDTHH-mm-ss.har`

---

## File Structure

### 新增文件
- `app/src/utils/har.js` - HAR 格式转换工具函数

### 修改文件
- `app/src-tauri/src/traffic.rs` - 添加 `get_batch` 方法
- `app/src-tauri/src/lib.rs` - 添加 `traffic_get_batch` Tauri 命令
- `app/src/stores/traffic.js` - 添加 `getDetailsBatch` 方法
- `app/src/pages/Monitor.vue` - 添加复选框、导出按钮、右键菜单、导出逻辑

### 测试文件
- `app/src-tauri/tests/traffic_batch.rs` - 批量查询接口测试

---

## Task 1: 后端批量查询接口

**Files:**
- Modify: `app/src-tauri/src/traffic.rs:179-181` (在 `SharedTraffic` impl 块中添加方法)
- Test: `app/src-tauri/tests/traffic_batch.rs` (新建测试文件)

**Interfaces:**
- Consumes: `SharedTraffic` 结构体，`TransactionDetail` 类型
- Produces: `SharedTraffic::get_batch(&self, ids: &[u64]) -> Vec<Option<TransactionDetail>>`

- [ ] **Step 1: 编写批量查询测试**

创建 `app/src-tauri/tests/traffic_batch.rs`:

```rust
//! 批量查询事务详情测试

use flowly_gui::traffic::SharedTraffic;

#[test]
fn test_get_batch_returns_details_in_order() {
    let traffic = SharedTraffic::new();
    
    // 模拟 3 个事务
    traffic.begin_request(
        1,
        "GET".to_string(),
        "http://example.com/1".to_string(),
        "example.com".to_string(),
        vec![("Host".to_string(), "example.com".to_string())],
        None,
        0,
        None,
    );
    traffic.complete(1, 200, None, vec![], None, 0, false);
    
    traffic.begin_request(
        2,
        "POST".to_string(),
        "http://example.com/2".to_string(),
        "example.com".to_string(),
        vec![],
        Some("data".to_string()),
        4,
        None,
    );
    traffic.complete(2, 201, None, vec![], None, 0, false);
    
    traffic.begin_request(
        3,
        "GET".to_string(),
        "http://example.com/3".to_string(),
        "example.com".to_string(),
        vec![],
        None,
        0,
        None,
    );
    traffic.complete(3, 404, None, vec![], None, 0, false);
    
    // 批量查询 [2, 1, 3] - 应该按请求顺序返回
    let batch = traffic.get_batch(&[2, 1, 3]);
    
    assert_eq!(batch.len(), 3);
    assert_eq!(batch[0].as_ref().unwrap().summary.method, "POST");
    assert_eq!(batch[1].as_ref().unwrap().summary.method, "GET");
    assert_eq!(batch[2].as_ref().unwrap().summary.status, Some(404));
}

#[test]
fn test_get_batch_handles_missing_ids() {
    let traffic = SharedTraffic::new();
    
    traffic.begin_request(
        10,
        "GET".to_string(),
        "http://example.com".to_string(),
        "example.com".to_string(),
        vec![],
        None,
        0,
        None,
    );
    traffic.complete(10, 200, None, vec![], None, 0, false);
    
    // 查询 [10, 999, 10] - 999 不存在
    let batch = traffic.get_batch(&[10, 999, 10]);
    
    assert_eq!(batch.len(), 3);
    assert!(batch[0].is_some());
    assert!(batch[1].is_none()); // 不存在的 ID
    assert!(batch[2].is_some());
}
```

- [ ] **Step 2: 运行测试验证失败**

运行: `cargo test --test traffic_batch -- --nocapture`
期望: FAIL，编译错误 "no method named `get_batch` found for struct `SharedTraffic`"

- [ ] **Step 3: 实现 get_batch 方法**

在 `app/src-tauri/src/traffic.rs` 的 `impl SharedTraffic` 块中（第 181 行后）添加:

```rust
    /// 批量获取完整事务（按 id 列表），保持顺序，缺失的条目为 None。
    pub fn get_batch(&self, ids: &[u64]) -> Vec<Option<TransactionDetail>> {
        ids.iter().map(|id| self.transactions.get(id)).collect()
    }
```

- [ ] **Step 4: 运行测试验证通过**

运行: `cargo test --test traffic_batch`
期望: PASS，2 个测试通过

- [ ] **Step 5: 提交**

```bash
git add app/src-tauri/src/traffic.rs app/src-tauri/tests/traffic_batch.rs
git commit -m "feat(backend): add batch query method for traffic details"
```

---

## Task 2: Tauri 命令注册

**Files:**
- Modify: `app/src-tauri/src/lib.rs:82-95` (添加命令函数和注册)

**Interfaces:**
- Consumes: `SharedTraffic::get_batch` 方法
- Produces: `traffic_get_batch` Tauri 命令，前端可通过 `invoke("traffic_get_batch", { ids })` 调用

- [ ] **Step 1: 添加 traffic_get_batch 命令函数**

在 `app/src-tauri/src/lib.rs` 中，找到其他 `traffic_*` 命令（约第 82-95 行），在其后添加:

```rust
#[tauri::command]
fn traffic_get_batch(
    traffic: tauri::State<'_, crate::traffic::SharedTraffic>,
    ids: Vec<u64>,
) -> Vec<Option<crate::traffic::TransactionDetail>> {
    traffic.get_batch(&ids)
}
```

- [ ] **Step 2: 注册命令到 invoke_handler**

在 `app/src-tauri/src/lib.rs` 的 `run` 函数中，找到 `.invoke_handler(tauri::generate_handler![...])`（约第 280 行），在列表中添加 `traffic_get_batch`:

```rust
.invoke_handler(tauri::generate_handler![
    config_get,
    config_set,
    proxy_status,
    proxy_start,
    proxy_stop,
    traffic_list,
    traffic_get,
    traffic_get_batch,  // 新增
    traffic_clear,
    rule_list,
    rule_set,
    intercept_list,
    intercept_decide,
    cert_path,
    cert_install,
    cert_status,
    system_proxy_status,
    system_proxy_set,
])
```

- [ ] **Step 3: 编译验证**

运行: `cargo build`
期望: 编译成功，无错误

- [ ] **Step 4: 提交**

```bash
git add app/src-tauri/src/lib.rs
git commit -m "feat(backend): register traffic_get_batch Tauri command"
```

---

## Task 3: 前端 traffic store 批量方法

**Files:**
- Modify: `app/src/stores/traffic.js:29-36` (在 actions 中添加方法)

**Interfaces:**
- Consumes: `traffic_get_batch` Tauri 命令
- Produces: `traffic.getDetailsBatch(ids)` 方法，返回 `Promise<Array<TransactionDetail>>`

- [ ] **Step 1: 添加 getDetailsBatch 方法**

在 `app/src/stores/traffic.js` 的 `actions` 对象中（约第 29 行，`getDetail` 方法后）添加:

```javascript
    /** 批量获取完整事务详情（按 id 数组），返回数组，顺序与 ids 一致，缺失的为 null。 */
    async getDetailsBatch(ids) {
      return await invoke("traffic_get_batch", { ids });
    },
```

- [ ] **Step 2: 验证方法可用**

在浏览器控制台测试（启动应用后）:

```javascript
// 假设当前有流量记录，id 为 [1, 2, 3]
const store = useTrafficStore();
const details = await store.getDetailsBatch([1, 2, 999]);
console.log(details); 
// 期望: [TransactionDetail, TransactionDetail, null]
```

- [ ] **Step 3: 提交**

```bash
git add app/src/stores/traffic.js
git commit -m "feat(frontend): add getDetailsBatch method to traffic store"
```

---

## Task 4: HAR 转换工具函数

**Files:**
- Create: `app/src/utils/har.js`

**Interfaces:**
- Consumes: `TransactionDetail` 对象（来自 `traffic.getDetailsBatch`）
- Produces: 
  - `transactionToHarEntry(txn)` - 单个事务转 HAR entry
  - `generateHarFile(entries)` - entries 数组转完整 HAR 文件对象

- [ ] **Step 1: 创建 har.js 工具文件**

创建 `app/src/utils/har.js`:

```javascript
/**
 * HAR (HTTP Archive) 格式转换工具
 * 规范: http://www.softwareishard.com/blog/har-12-spec/
 */

/**
 * 解析 URL 查询参数为 HAR queryString 数组
 * @param {string} url - 完整 URL
 * @returns {Array<{name: string, value: string}>}
 */
function parseQueryString(url) {
  try {
    const params = new URL(url).searchParams;
    return Array.from(params.entries()).map(([name, value]) => ({ name, value }));
  } catch {
    return [];
  }
}

/**
 * 从 headers 数组提取指定 header 的值
 * @param {Array<[string, string]>} headers - [[name, value], ...]
 * @param {string} headerName - header 名称（不区分大小写）
 * @returns {string} header 值，不存在返回默认值
 */
function getHeader(headers, headerName, defaultValue = "") {
  const header = headers.find(([k]) => k.toLowerCase() === headerName.toLowerCase());
  return header ? header[1] : defaultValue;
}

/**
 * 将 TransactionDetail 转换为 HAR entry
 * @param {Object} txn - TransactionDetail 对象
 * @returns {Object} HAR entry 对象
 */
export function transactionToHarEntry(txn) {
  return {
    startedDateTime: new Date(Number(txn.summary.startedAt)).toISOString(),
    time: Number(txn.summary.durationMs),
    request: {
      method: txn.summary.method,
      url: txn.summary.url,
      httpVersion: "HTTP/1.1",
      headers: txn.reqHeaders.map(([name, value]) => ({ name, value })),
      queryString: parseQueryString(txn.summary.url),
      cookies: [],
      headersSize: -1,
      bodySize: txn.summary.reqSize || 0,
      postData: txn.reqBody
        ? {
            mimeType: getHeader(txn.reqHeaders, "content-type", "application/octet-stream"),
            text: txn.reqBody,
          }
        : undefined,
    },
    response: {
      status: txn.summary.status || 0,
      statusText: "",
      httpVersion: "HTTP/1.1",
      headers: txn.resHeaders.map(([name, value]) => ({ name, value })),
      cookies: [],
      content: {
        size: txn.summary.resSize || 0,
        mimeType: getHeader(txn.resHeaders, "content-type", "application/octet-stream"),
        text: txn.resBody || "",
      },
      redirectURL: "",
      headersSize: -1,
      bodySize: txn.summary.resSize || 0,
    },
    cache: {},
    timings: {
      send: 0,
      wait: Number(txn.summary.durationMs),
      receive: 0,
    },
  };
}

/**
 * 生成完整的 HAR 文件对象
 * @param {Array<Object>} entries - HAR entry 数组
 * @returns {Object} 完整的 HAR 文件对象
 */
export function generateHarFile(entries) {
  return {
    log: {
      version: "1.2",
      creator: {
        name: "Flowly Proxy",
        version: "1.0.0",
      },
      entries: entries,
    },
  };
}
```

- [ ] **Step 2: 验证转换函数**

在浏览器控制台测试:

```javascript
import { transactionToHarEntry, generateHarFile } from './utils/har.js';

// 模拟 TransactionDetail
const mockTxn = {
  summary: {
    method: "GET",
    url: "http://example.com/test?key=value",
    status: 200,
    startedAt: Date.now(),
    durationMs: 123,
    reqSize: 0,
    resSize: 100,
  },
  reqHeaders: [["Host", "example.com"]],
  reqBody: null,
  resHeaders: [["Content-Type", "text/html"]],
  resBody: "<html>test</html>",
};

const entry = transactionToHarEntry(mockTxn);
console.log(entry);
// 期望: 符合 HAR entry 规范的对象

const har = generateHarFile([entry]);
console.log(JSON.stringify(har, null, 2));
// 期望: 完整的 HAR 文件 JSON
```

- [ ] **Step 3: 提交**

```bash
git add app/src/utils/har.js
git commit -m "feat(frontend): add HAR format conversion utilities"
```

---

## Task 5: Monitor.vue 添加复选框和导出按钮

**Files:**
- Modify: `app/src/pages/Monitor.vue:37-43` (表格添加复选框列)
- Modify: `app/src/pages/Monitor.vue:26-35` (工具栏添加导出按钮)
- Modify: `app/src/pages/Monitor.vue:71-171` (script 添加状态和方法)

**Interfaces:**
- Consumes: `traffic.getDetailsBatch`, `transactionToHarEntry`, `generateHarFile`
- Produces: 可选择的表格、导出按钮、右键菜单

- [ ] **Step 1: 添加复选框列到表格**

在 `app/src/pages/Monitor.vue` 的 `<el-table>` 标签中（约第 37 行）添加 `@selection-change` 事件:

```vue
<el-table
  :data="filteredList"
  stripe
  size="small"
  height="calc(100vh - 130px)"
  @row-click="openDetail"
  @selection-change="handleSelectionChange"
  @row-contextmenu="handleContextMenu"
  empty-text="暂无流量。请到「代理设置」启动代理，并将系统/浏览器代理设为监听地址。"
>
  <el-table-column type="selection" width="55" />
  <!-- 现有列保持不变 -->
```

- [ ] **Step 2: 在工具栏添加导出按钮**

在 `app/src/pages/Monitor.vue` 的工具栏（约第 33 行，"加载历史"按钮位置）添加导出按钮:

```vue
<el-input
  v-model="filterText"
  placeholder="URL 过滤（输入 URL 包含词）"
  clearable
  size="small"
  class="filter-input"
/>
<el-button
  size="small"
  type="success"
  :disabled="selectedRows.length === 0"
  :loading="exporting"
  @click="exportToHar"
>
  {{ exporting ? "导出中..." : `导出${selectedRows.length > 0 ? ` (${selectedRows.length})` : ""}` }}
</el-button>
<el-button size="small" type="danger" plain @click="clear">清空</el-button>
```

- [ ] **Step 3: 添加右键菜单**

在 `app/src/pages/Monitor.vue` 的 `</el-table>` 标签后（约第 67 行前）添加右键菜单:

```vue
</el-table>

<!-- 右键菜单 -->
<div
  v-if="contextMenuVisible"
  class="context-menu"
  :style="{ left: contextMenuX + 'px', top: contextMenuY + 'px' }"
>
  <div class="context-menu-item" @click="exportToHar">导出选中为 HAR</div>
</div>

<DetailDrawer v-model="drawerVisible" :id="selectedId" />
```

- [ ] **Step 4: 添加样式**

在 `app/src/pages/Monitor.vue` 的 `<style scoped>` 中（约第 212 行前）添加:

```css
.context-menu {
  position: fixed;
  background: white;
  border: 1px solid #dcdfe6;
  border-radius: 4px;
  box-shadow: 0 2px 12px 0 rgba(0, 0, 0, 0.1);
  z-index: 9999;
  padding: 6px 0;
}

.context-menu-item {
  padding: 8px 16px;
  cursor: pointer;
  font-size: 14px;
}

.context-menu-item:hover {
  background: #f5f7fa;
}
```

- [ ] **Step 5: 添加状态和方法到 script**

在 `app/src/pages/Monitor.vue` 的 `<script setup>` 中（约第 78 行，`const traffic = useTrafficStore();` 后）添加状态:

```javascript
const traffic = useTrafficStore();
const running = ref(false);
const busy = ref(false);
const drawerVisible = ref(false);
const selectedId = ref(null);

// 导出相关状态
const selectedRows = ref([]);
const exporting = ref(false);
const contextMenuVisible = ref(false);
const contextMenuX = ref(0);
const contextMenuY = ref(0);
```

- [ ] **Step 6: 添加处理方法**

在 `app/src/pages/Monitor.vue` 的 script 中（约第 164 行，`clear` 函数前）添加:

```javascript
function handleSelectionChange(selection) {
  selectedRows.value = selection;
}

function handleContextMenu(row, column, event) {
  event.preventDefault();
  // 只在选中的行上显示右键菜单
  if (selectedRows.value.some((r) => r.id === row.id)) {
    contextMenuX.value = event.clientX;
    contextMenuY.value = event.clientY;
    contextMenuVisible.value = true;
  }
}

// 点击其他地方关闭右键菜单
document.addEventListener("click", () => {
  contextMenuVisible.value = false;
});

async function exportToHar() {
  if (selectedRows.value.length === 0) return;
  
  contextMenuVisible.value = false;
  exporting.value = true;
  
  try {
    // 生成默认文件名
    const timestamp = new Date().toISOString().replace(/[:.]/g, "-").slice(0, 19);
    const defaultName = `traffic_${timestamp}.har`;
    
    // 弹出保存对话框
    const filePath = await save({
      defaultPath: defaultName,
      filters: [{ name: "HAR 文件", extensions: ["har"] }],
    });
    
    if (!filePath) return; // 用户取消
    
    // 批量获取详情
    const ids = selectedRows.value.map((r) => r.id);
    const details = await traffic.getDetailsBatch(ids);
    
    // 过滤掉 null（已过期）并转换
    const entries = details.filter((d) => d !== null).map((txn) => transactionToHarEntry(txn));
    
    if (entries.length === 0) {
      ElMessage.warning("选中的会话已全部过期，无法导出");
      return;
    }
    
    // 生成 HAR 并写入文件
    const har = generateHarFile(entries);
    await writeTextFile(filePath, JSON.stringify(har, null, 2));
    
    ElMessage.success(`导出成功：${entries.length} 个会话`);
  } catch (e) {
    ElMessage.error(`导出失败：${e}`);
  } finally {
    exporting.value = false;
  }
}
```

- [ ] **Step 7: 添加 import 语句**

在 `app/src/pages/Monitor.vue` 的 `<script setup>` 顶部（约第 72 行）添加:

```javascript
import { ref, computed, onMounted } from "vue";
import { ElMessage } from "element-plus";
import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import { writeTextFile } from "@tauri-apps/plugin-fs";
import { useTrafficStore } from "../stores/traffic";
import DetailDrawer from "../components/DetailDrawer.vue";
import { transactionToHarEntry, generateHarFile } from "../utils/har";
```

- [ ] **Step 8: 手动测试 UI 交互**

启动应用，测试以下场景:

1. 未选中时，导出按钮灰色禁用
2. 勾选 1 个或多个会话，导出按钮高亮，显示"导出 (N)"
3. 点击导出按钮，弹出保存对话框，默认文件名为 `traffic_2026-08-24T14-30-15.har`
4. 选择保存位置，点击保存，显示成功提示
5. 右键点击选中的行，显示右键菜单
6. 点击"导出选中为 HAR"，执行导出流程

- [ ] **Step 9: 提交**

```bash
git add app/src/pages/Monitor.vue
git commit -m "feat(frontend): add checkbox selection, export button, and context menu to Monitor"
```

---

## Task 6: 安装 Tauri 插件依赖

**Files:**
- Modify: `app/package.json`
- Modify: `app/src-tauri/Cargo.toml`
- Modify: `app/src-tauri/src/lib.rs` (插件注册)

**Interfaces:**
- Consumes: Tauri Dialog Plugin, Tauri FS Plugin
- Produces: `save` 和 `writeTextFile` API 可用

- [ ] **Step 1: 安装前端插件**

运行:
```bash
cd app
npm install @tauri-apps/plugin-dialog @tauri-apps/plugin-fs
```

- [ ] **Step 2: 安装后端插件**

运行:
```bash
cd app/src-tauri
cargo add tauri-plugin-dialog tauri-plugin-fs
```

- [ ] **Step 3: 注册插件到 Tauri Builder**

在 `app/src-tauri/src/lib.rs` 的 `run` 函数中（约第 260 行），找到 `tauri::Builder::default()`，添加插件:

```rust
tauri::Builder::default()
    .plugin(tauri_plugin_dialog::init())
    .plugin(tauri_plugin_fs::init())
    .setup(|app| {
        // ... 现有 setup 代码
    })
```

- [ ] **Step 4: 编译验证**

运行: `cargo build`
期望: 编译成功，无错误

- [ ] **Step 5: 提交**

```bash
git add app/package.json app/package-lock.json app/src-tauri/Cargo.toml app/src-tauri/Cargo.lock app/src-tauri/src/lib.rs
git commit -m "feat: install Tauri dialog and fs plugins for file operations"
```

---

## Task 7: 集成测试

**Files:**
- 无新增文件，使用现有功能

**Interfaces:**
- Consumes: 完整的导出功能
- Produces: 验证导出功能正确性

- [ ] **Step 1: 启动应用并捕获流量**

```bash
cd app
npm run tauri dev
```

在浏览器中访问几个网站，确保有流量记录

- [ ] **Step 2: 测试单选导出**

1. 勾选 1 个会话
2. 点击"导出 (1)"
3. 保存文件
4. 用文本编辑器打开 HAR 文件，验证:
   - `log.version` 为 "1.2"
   - `log.entries` 有 1 个元素
   - `entries[0].request` 和 `response` 结构正确
   - `entries[0].startedDateTime` 为 ISO 格式

- [ ] **Step 3: 测试多选导出**

1. 勾选 3-5 个会话
2. 点击"导出 (N)"
3. 保存文件
4. 验证 HAR 文件中 `entries` 数量与选中数量一致

- [ ] **Step 4: 测试会话过期**

1. 勾选几个会话
2. 等待 60 秒（事务缓存 TTL）
3. 点击导出
4. 验证提示信息："X 个会话已过期"或成功导出剩余会话

- [ ] **Step 5: 测试用户取消**

1. 勾选会话
2. 点击导出
3. 在保存对话框中点击"取消"
4. 验证无错误提示，按钮恢复正常

- [ ] **Step 6: 测试右键菜单**

1. 勾选几个会话
2. 右键点击选中的行
3. 点击"导出选中为 HAR"
4. 验证导出流程正常执行

- [ ] **Step 7: 验证 HAR 文件规范**

使用在线 HAR 验证工具（如 http://www.softwareishard.com/blog/har-12-spec/）验证导出的文件符合 HAR 1.2 规范

- [ ] **Step 8: 最终提交**

```bash
git add .
git commit -m "test: verify HAR export functionality"
```

---

## Task 8: 清理和优化

**Files:**
- 可能修改: `app/src/pages/Monitor.vue` (根据测试反馈优化)

**Interfaces:**
- 无新接口

- [ ] **Step 1: 代码审查**

检查所有修改的文件:
- 代码风格是否一致
- 是否有冗余代码
- 错误处理是否完善
- 注释是否清晰

- [ ] **Step 2: 性能优化**

验证批量导出的性能:
- 选择 100+ 个会话，验证导出速度
- 检查内存使用情况
- 优化大文件写入（如果需要）

- [ ] **Step 3: 用户体验优化**

根据测试结果优化:
- 按钮文案是否清晰
- 提示信息是否友好
- 加载状态是否明显
- 错误提示是否有帮助

- [ ] **Step 4: 最终提交**

```bash
git add .
git commit -m "refactor: optimize HAR export based on testing feedback"
```

---

## 完成标准

- ✅ 后端批量查询接口正常工作
- ✅ 前端复选框多选功能正常
- ✅ 导出按钮和右键菜单可用
- ✅ HAR 文件格式符合规范
- ✅ 文件保存对话框正常弹出
- ✅ 错误处理和边界情况处理完善
- ✅ 性能表现良好（100+ 会话导出不超过 5 秒）

---

## 风险点

1. **Tauri 插件版本兼容性**: 确保 `@tauri-apps/plugin-dialog` 和 `@tauri-apps/plugin-fs` 版本与项目使用的 Tauri 版本兼容
2. **大文件写入**: 如果导出大量会话（1000+），可能需要考虑流式写入而非一次性写入
3. **会话过期**: 事务缓存 TTL 为 60 秒，长时间不操作后导出可能部分会话已过期
4. **右键菜单冲突**: 确保右键菜单不与浏览器默认行为冲突

---

## 后续优化

- 支持按域名/状态码/时间范围过滤后导出
- 支持导出为其他格式（Postman Collection、OpenAPI）
- 支持保存导出配置为预设
- 支持拖拽排序导出顺序
- 支持导出时包含注释/标签
