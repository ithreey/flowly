# 发送器（Request Sender）设计文档

## 概述

在 Flowly 桌面端新增"发送器"页面，提供类似 Postman 的 HTTP 请求构造与调试能力。用户可手动构造任意 HTTP 请求，查看格式化响应，选择是否经过 Flowly 代理（使规则生效）。

## 定位

- 独立 API 调试工具
- MVP 阶段：请求发送 + 历史记录
- 后续迭代：集合管理（Collections）、Auth 配置

## 页面布局

采用左右分栏方案：

```
┌──────────┬───────────────────────────────────────┐
│ 历史记录  │  ┌──────────────────────────────────┐ │
│          │  │ [POST ▼] [https://...] [Send ▶]  │ │
│ 🔍 搜索   │  │ ☐ 经过代理                       │ │
│          │  ├──────────────────────────────────┤ │
│ ─ 今天 ─ │  │ Params | Headers | Body | Auth   │ │
│ GET /us… │  │                                  │ │
│ 200 123ms│  │  (请求编辑区)                     │ │
│          │  │                                  │ │
│ ─ 昨天 ─ │  ├──────────────────────────────────┤ │
│ POST /l… │  │ Response    200 OK   123ms 1.2KB│ │
│ 201 45ms │  │ Body | Headers | Preview         │ │
│          │  │                                  │ │
│ GET /he… │  │  (响应查看区)                     │ │
│ 200 89ms │  │                                  │ │
│          │  └──────────────────────────────────┘ │
│ [清空历史]│                                       │
└──────────┴───────────────────────────────────────┘
```

- 左栏宽度 240px，可折叠到图标模式
- 右栏自适应剩余宽度
- 历史记录按日期分组（今天 / 昨天 / 更早）
- 左侧顶部有搜索框过滤历史

## 菜单入口

- 侧边栏新增"发送器"菜单项
- 路由：`/sender`
- 图标：`Promotion`（Element Plus 图标）
- 页面标题："发送器"
- 页面描述："构造和发送 HTTP 请求，调试 API 接口"

## 请求编辑区

### URL 行

- `el-select` 选择 HTTP 方法：GET / POST / PUT / PATCH / DELETE / OPTIONS / HEAD
- `el-input` 输入完整 URL
- `el-button` 发送按钮（带 loading 状态）
- URL 下方一行：`☐ 经过代理` 开关（默认勾选），右侧显示响应状态摘要

### Tab 面板

四个 Tab：Params / Headers / Body / Auth

#### Params Tab

- Key-Value 表格，每行：checkbox（启用）+ key input + value input + 删除按钮
- 底部 `+ 添加参数` 按钮
- 参数自动拼接到 URL query string

#### Headers Tab

- 同 Params 的 Key-Value 表格模式
- 预填常用默认头：`Accept: */*`、`Content-Type`（根据 Body 类型自动设置）
- 用户可删除/修改/新增

#### Body Tab

顶部 radio 切换：`none` / `form-data` / `x-www-form-urlencoded` / `raw`

- `none`：空提示"此请求没有 Body"
- `form-data`：Key-Value 表格，value 列右侧有类型切换（文本/文件），文件类型时显示文件选择按钮
- `x-www-form-urlencoded`：Key-Value 表格，纯文本键值对
- `raw`：文本编辑器（CodeMirror）+ 格式下拉（Text / JSON / XML / HTML）
  - JSON 格式时自动语法高亮，发送前校验格式
  - 自动设置 `Content-Type` 头

#### Auth Tab

- MVP 阶段显示"即将支持"占位提示
- 后续扩展：Basic Auth / Bearer Token / API Key

## 响应查看区

### 状态栏

- 状态码（带颜色：2xx 绿 / 3xx 蓝 / 4xx 黄 / 5xx 红）
- 状态文本（OK / Not Found / ...）
- 耗时（ms）
- 响应大小（KB）
- 未发送时显示空状态提示："点击 Send 发送请求"

### Tab 面板

三个 Tab：Body / Headers / Preview

#### Body Tab

子 Tab 切换：`Pretty` / `Raw`

- `Pretty`：
  - JSON：CodeMirror 格式化高亮（复用现有 CodeMirror 依赖）
  - XML：格式化缩进
  - 其他：纯文本等宽字体
- `Raw`：原始响应文本，等宽字体，无格式化

#### Headers Tab

- 响应头列表，Key-Value 表格形式
- 顶部显示状态行（如 `HTTP/1.1 200 OK`）

#### Preview Tab

- HTML：渲染到 sandboxed iframe
- 图片（png/jpg/gif/svg/webp）：直接 `<img>` 渲染
- 其他格式：显示"此内容类型不支持预览"

### Loading / Error 状态

- 发送中：loading 动画 + 可取消按钮
- 网络错误：红色 `el-alert` 提示（连接超时 / DNS 失败 / 证书错误等）
- 超时：30s 超时提示

## 历史记录

### 存储

- 后端持久化到 `data_dir/history.json`
- 每条记录字段：

```rust
struct HistoryEntry {
    id: u64,
    method: String,
    url: String,
    headers: Vec<(String, String)>,
    body: Option<Vec<u8>>,
    through_proxy: bool,
    status: u16,
    status_text: String,
    response_headers: Vec<(String, String)>,
    response_body: Option<Vec<u8>>,
    duration_ms: u64,
    timestamp: i64,  // Unix timestamp
}
```

- 最多保存 200 条，超出自动淘汰最旧的

### 左侧列表

- 每条显示：方法标签（彩色）+ URL 路径（截断显示）+ 状态码 + 耗时
- 按日期分组（今天 / 昨天 / 更早）
- 顶部搜索框：按 URL 关键词过滤
- 点击某条记录 → 回填到右侧请求编辑区和响应查看区
- 底部 `清空历史` 按钮（二次确认对话框）

### 右键菜单

- 删除单条记录
- 复制为 cURL 命令

## 后端设计

### 新增 Tauri 命令

#### `send_request`

```rust
#[tauri::command]
async fn send_request(
    state: State<'_, AppState>,
    method: String,
    url: String,
    headers: Vec<(String, String)>,
    body: Option<Vec<u8>>,
    through_proxy: bool,
) -> Result<SendResponse, String>
```

返回结构：

```rust
struct SendResponse {
    status: u16,
    status_text: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    duration_ms: u64,
}
```

请求路由逻辑：
- `through_proxy = false` → `gen_client(None)` 直连目标
- `through_proxy = true` → 构造指向自身 `listen_addr` 的代理客户端（复用 `proxy_ctrl.rs` 中的代理构造模式），请求经过 Flowly 代理，规则生效，自动出现在流量监控中

#### 历史命令

- `history_list() -> Vec<HistoryEntry>` 返回历史列表
- `history_save(entry: HistoryEntry)` 保存一条记录
- `history_clear()` 清空所有记录
- `history_delete(id: u64)` 删除单条记录

### 技术复用

- HTTP 客户端：复用 `mitm_core::http_client::gen_client`
- 请求构建：复用 `replay_traffic_request` 的模式（hyper Request 构建、header 清理）
- TLS：复用 `TrustAllCertVerifier`（信任所有证书，代理场景需要）
- CodeMirror：复用现有依赖（`@codemirror/lang-json`、`@codemirror/lang-xml`）

## 前端文件结构

新增文件：

```
app/src/
├── pages/
│   └── Sender.vue           # 发送器页面
├── components/
│   ├── RequestBuilder.vue    # 请求编辑组件（URL + Tabs）
│   ├── ResponseViewer.vue    # 响应查看组件（Status + Tabs）
│   └── HistoryPanel.vue      # 历史记录面板
├── stores/
│   └── sender.js             # Pinia store（当前请求状态、历史）
└── utils/
    └── curl.js               # 复制为 cURL 命令工具函数
```

修改文件：

```
app/src/
├── router.js                 # 添加 /sender 路由
├── App.vue                   # 添加侧边栏菜单项 + 页面描述
└── src-tauri/src/
    ├── lib.rs                # 注册新命令
    ├── sender.rs             # send_request 命令实现
    └── history.rs            # 历史记录命令实现
```

## 后续迭代（不在 MVP 范围）

- 集合管理（Collections）：创建文件夹、分组保存请求
- Auth 配置：Basic Auth / Bearer Token / API Key
- 环境变量：定义 base URL 变量，不同环境切换
- 请求导入：从 cURL / HAR / Postman Collection 导入
- WebSocket 支持
