# Flowly

[English](./README_us.md)

Flowly 是一个面向本机调试的 HTTP/HTTPS 代理与 MITM 工具。它由 Rust 代理核心和 Tauri + Vue 桌面工作台组成，用于查看网络会话、调试请求与响应、按规则拦截流量，以及导出 HAR 数据。

> 仅对你拥有或明确获授权的流量使用 MITM、请求修改和证书解密功能。

## 当前能力

### 流量监控

![流量监控](./assets/screenshot-traffic-monitor.png)

- 启动或停止本地 HTTP/HTTPS 代理
- 默认监听地址为 `127.0.0.1:34567`
- 查看请求方法、状态码、URL、请求/响应大小、耗时和时间
- 按 HTTP 方法和 URL 关键词筛选
- 选择会话后批量删除或清空记录
- 打开详情查看请求头、响应头、请求体和响应体
- 复制 URL 或生成 cURL 命令
- 将选中的会话导出为 HAR 文件
- 通过右键菜单重放请求
- 实时显示会话状态（等待中、成功、失败），带有状态图标和加载动画

![请求详情检查器](./assets/screenshot-detail-inspector.png)

代理启动后，客户端需要使用 Flowly 的监听地址作为 HTTP 代理。桌面端可以按配置在启动时自动设置系统代理，并在停止时还原；该功能当前主要针对 Windows。

### 规则配置

![规则配置](./assets/screenshot-rule-config.png)

规则可以通过表单创建，也可以使用 JSON 高级编辑器。保存后会立即持久化并热应用到代理。

支持的匹配条件包括：

- 全部请求
- 精确域名
- 域名关键词
- 域名前缀
- 域名后缀
- URL 包含
- URL 正则表达式

支持的动作包括：

- 拒绝请求并返回 `502`
- 重定向
- 拦截并等待确认
- 记录请求或响应
- 修改响应体

规则还可以通过 `mitmList` 指定需要解密的 HTTPS 域名范围。规则文件使用 JSON，桌面端支持导入单条规则或规则数组。

示例：

```json
{
  "name": "记录示例域名请求",
  "enabled": true,
  "mitmList": "*.example.com",
  "filters": [
    { "domainSuffix": "example.com" }
  ],
  "actions": [
    "logReq"
  ]
}
```

### CA 证书管理

![证书管理](./assets/screenshot-cert-manager.png)

HTTPS MITM 需要本地根证书。Flowly 桌面端可以：

- 查看当前 CA 证书状态
- 生成或重新生成 CA 证书
- 安装到系统信任区
- 复制 PEM 证书内容
- 通过代理地址提供证书下载入口：`http://<代理地址>/mitm/cert`

重新生成证书会使之前安装的信任关系失效，需要重新安装。

### 代理与应用设置

- 配置监听地址
- 配置可选的上游代理
- 设置是否自动接管系统代理
- 设置是否采集请求/响应体及最大采集大小
- 调整桌面工作台字体大小

## 使用桌面版

### 环境要求

- Windows
- Rust 工具链
- Node.js 和 npm
- Tauri 2 的本地构建依赖

### 安装依赖

```powershell
cd app
npm install
```

### 启动开发版

```powershell
cd app
npm run tauri dev
```

开发前端单独启动：

```powershell
cd app
npm run dev
```

默认地址为 `http://localhost:1420`。仅运行 Vite 只能预览界面，代理控制、证书和文件操作等 Tauri 能力需要通过 `npm run tauri dev` 使用。

### 构建桌面安装包

```powershell
cd app
npm run tauri build
```

前端生产构建：

```powershell
cd app
npm run build
```

## 使用 Rust CLI

仓库也保留了不依赖桌面界面的 Rust CLI。CLI 需要先准备 CA 私钥和证书。

### 生成 CA

```powershell
cargo run -- genca
```

生成的默认文件位于：

- `ca/private.key`
- `ca/cert.crt`

### 启动代理

```powershell
cargo run -- run -r rules/modify.json
```

常用参数：

```text
--key,  -k    CA 私钥路径，默认 ca/private.key
--cert, -c    CA 证书路径，默认 ca/cert.crt
--rule, -r    规则 JSON 文件或目录
--bind, -b    监听地址，默认 127.0.0.1:34567
--proxy, -p   可选上游代理，例如 http://127.0.0.1:7890
```

例如：

```powershell
cargo run -- run `
  --key ca/private.key `
  --cert ca/cert.crt `
  --rule rules `
  --bind 127.0.0.1:34567 `
  --proxy http://127.0.0.1:7890
```

CLI 规则加载支持单个 JSON 文件或目录。目录中的规则文件会被合并加载。

## HTTPS 使用说明

1. 生成 Flowly CA 证书。
2. 将 CA 证书安装到操作系统或目标浏览器的信任区。
3. 启动 Flowly 代理。
4. 将浏览器或客户端的 HTTP/HTTPS 代理设置为 `127.0.0.1:34567`。
5. 在流量监控页面查看会话。

如果只需要查看 HTTP 流量，可以不安装 CA 证书。要解密 HTTPS，目标域名必须包含在规则的 `mitmList` 中，并且客户端必须信任 Flowly CA。

## 项目结构

```text
.
├── app/                 Tauri + Vue 桌面工作台
│   ├── src/             页面、组件和前端状态
│   └── src-tauri/       代理控制、证书、规则和系统代理命令
├── crates/core/         HTTP/HTTPS 代理与 MITM 核心
├── crates/rule/         规则匹配和请求/响应处理
├── crates/trust_cert/   系统信任区证书安装
├── src/                 Rust CLI 入口
├── ca/                  本地 CA 文件目录
└── rules/               示例规则
```

## 注意事项

- MITM 会生成目标域名的临时证书，必须获得客户端信任才能正常访问 HTTPS。
- 采集请求/响应体会增加内存使用，桌面端可以设置最大采集大小。
- 规则保存后会立即生效，错误的规则可能导致请求被拒绝或内容被改写。
- 停止代理前确认系统代理已经恢复；Flowly 会在正常停止时执行还原逻辑。
- 透明代理不是桌面端的默认工作流，需要根据操作系统自行配置转发规则。

## 许可证

Flowly 使用 MIT License，详见 [LICENSE](./LICENSE)。
