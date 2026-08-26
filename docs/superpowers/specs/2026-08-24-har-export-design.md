# HAR 导出功能设计文档

**日期**: 2026-08-24  
**状态**: 已确认  
**目标**: 为流量监控页面添加会话导出功能，支持多选会话并导出为 HAR 文件

---

## 1. 需求概述

在流量监控页面（Monitor.vue）中添加 HAR 导出功能，允许用户：
- 通过复选框选择多个会话
- 导出选中的会话为标准 HAR (HTTP Archive) 格式文件
- 自定义保存位置和文件名

---

## 2. 用户交互流程

### 2.1 核心流程

1. 表格每行显示复选框，用户点击勾选要导出的会话
2. 工具栏"导出"按钮根据选中状态启用/禁用
3. 点击导出按钮（或右键菜单）→ 弹出保存对话框（默认文件名含时间戳）
4. 用户确认保存位置 → 后端批量获取数据 → 生成 HAR → 显示成功提示

### 2.2 UI 状态

- **未选中时**：导出按钮灰色禁用
- **选中时**：导出按钮高亮，显示"导出 N 个会话"
- **导出中**：按钮显示 loading 状态
- **导出完成**：消息提示"导出成功"

### 2.3 右键菜单

- 选中会话后，右键点击任意选中行 → 弹出菜单 → "导出选中为 HAR"

---

## 3. 前端组件改动

### 3.1 Monitor.vue 改动

#### 表格添加复选框列

```vue
<el-table @selection-change="handleSelectionChange">
  <el-table-column type="selection" width="55" />
  <!-- 现有列 -->
</el-table>
```

#### 工具栏导出按钮

```vue
<el-button 
  size="small" 
  type="success" 
  :disabled="selectedRows.length === 0"
  :loading="exporting"
  @click="exportToHar"
>
  {{ exporting ? '导出中...' : `导出${selectedRows.length > 0 ? ` (${selectedRows.length})` : ''}` }}
</el-button>
```

#### 右键菜单

```vue
<el-table @row-contextmenu="showContextMenu">
  <!-- 右键菜单组件 -->
</el-table>
```

#### 新增状态和方法

- `selectedRows`: 存储选中的行数据
- `exporting`: 导出状态
- `handleSelectionChange`: 处理选择变化
- `exportToHar`: 导出逻辑
- `showContextMenu`: 显示右键菜单

### 3.2 traffic store 新增方法

```javascript
async getDetailsBatch(ids) {
  return await invoke("traffic_get_batch", { ids });
}
```

---

## 4. 后端新增接口

### 4.1 traffic.rs 新增方法

在 `SharedTraffic` 结构体中添加批量查询方法：

```rust
impl SharedTraffic {
    /// 批量获取完整事务（按 id 列表）
    pub fn get_batch(&self, ids: &[u64]) -> Vec<Option<TransactionDetail>> {
        ids.iter().map(|id| self.transactions.get(id)).collect()
    }
}
```

### 4.2 lib.rs 新增 Tauri 命令

```rust
#[tauri::command]
fn traffic_get_batch(
    traffic: State<'_, SharedTraffic>,
    ids: Vec<u64>,
) -> Vec<Option<TransactionDetail>> {
    traffic.get_batch(&ids)
}

// 注册命令
.invoke_handler(tauri::generate_handler![
    // ... 现有命令
    traffic_get_batch,
])
```

### 4.3 性能考虑

- `moka::sync::Cache::get()` 是并发安全的，可以并行查询
- 一次 IPC 调用完成，避免前端循环调用的延迟累积
- 返回 `Vec<Option<...>>` 保持与 id 列表的顺序对应，缺失的条目为 `None`

---

## 5. HAR 格式转换

### 5.1 HAR 结构映射

```javascript
// TransactionDetail → HAR entry
function transactionToHarEntry(txn) {
  return {
    startedDateTime: new Date(txn.summary.startedAt).toISOString(),
    time: txn.summary.durationMs,
    request: {
      method: txn.summary.method,
      url: txn.summary.url,
      httpVersion: "HTTP/1.1",
      headers: txn.reqHeaders.map(([name, value]) => ({ name, value })),
      queryString: parseQueryString(txn.summary.url),
      cookies: [],
      headersSize: -1,
      bodySize: txn.summary.reqSize,
      postData: txn.reqBody ? {
        mimeType: getContentType(txn.reqHeaders),
        text: txn.reqBody
      } : undefined
    },
    response: {
      status: txn.summary.status || 0,
      statusText: "",
      httpVersion: "HTTP/1.1",
      headers: txn.resHeaders.map(([name, value]) => ({ name, value })),
      cookies: [],
      content: {
        size: txn.summary.resSize,
        mimeType: getContentType(txn.resHeaders),
        text: txn.resBody || ""
      },
      redirectURL: "",
      headersSize: -1,
      bodySize: txn.summary.resSize
    },
    cache: {},
    timings: {
      send: 0,
      wait: txn.summary.durationMs,
      receive: 0
    }
  };
}
```

### 5.2 辅助函数

```javascript
// 解析 URL 查询参数
function parseQueryString(url) {
  try {
    const params = new URL(url).searchParams;
    return Array.from(params.entries()).map(([name, value]) => ({ name, value }));
  } catch {
    return [];
  }
}

// 从 headers 提取 Content-Type
function getContentType(headers) {
  const header = headers.find(([k]) => k.toLowerCase() === 'content-type');
  return header ? header[1] : 'application/octet-stream';
}
```

### 5.3 完整 HAR 文件

```javascript
function generateHarFile(entries) {
  return {
    log: {
      version: "1.2",
      creator: {
        name: "Flowly Proxy",
        version: "1.0.0"
      },
      entries: entries
    }
  };
}
```

---

## 6. 文件保存和错误处理

### 6.1 文件保存流程

```javascript
import { save } from '@tauri-apps/plugin-dialog';
import { writeTextFile } from '@tauri-apps/plugin-fs';

async function exportToHar() {
  if (selectedRows.value.length === 0) return;
  
  exporting.value = true;
  try {
    // 1. 生成默认文件名
    const timestamp = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19);
    const defaultName = `traffic_${timestamp}.har`;
    
    // 2. 弹出保存对话框
    const filePath = await save({
      defaultPath: defaultName,
      filters: [{ name: 'HAR 文件', extensions: ['har'] }]
    });
    
    if (!filePath) return; // 用户取消
    
    // 3. 批量获取详情
    const ids = selectedRows.value.map(r => r.id);
    const details = await traffic.getDetailsBatch(ids);
    
    // 4. 过滤掉 None（已过期）并转换
    const entries = details
      .filter(d => d !== null)
      .map(txn => transactionToHarEntry(txn));
    
    // 5. 生成 HAR 并写入文件
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

### 6.2 错误处理

- **会话过期**：批量接口返回 `None`，前端过滤并提示"X 个会话已过期"
- **文件写入失败**：捕获异常并显示错误消息
- **用户取消**：保存对话框返回 null，直接返回不报错

### 6.3 边界情况

- **选中 0 项**：按钮禁用，不触发导出
- **所有会话过期**：提示"选中的会话已全部过期，无法导出"
- **大数据量**：批量接口性能良好，无需特殊处理

---

## 7. 实现清单

- [ ] 后端：在 `SharedTraffic` 添加 `get_batch` 方法
- [ ] 后端：添加 `traffic_get_batch` Tauri 命令
- [ ] 后端：注册新命令到 `invoke_handler`
- [ ] 前端：traffic store 添加 `getDetailsBatch` 方法
- [ ] 前端：Monitor.vue 添加复选框列
- [ ] 前端：Monitor.vue 添加导出按钮和右键菜单
- [ ] 前端：实现 HAR 转换逻辑
- [ ] 前端：实现文件保存流程
- [ ] 测试：验证导出功能
- [ ] 测试：验证边界情况（0 选中、会话过期等）

---

## 8. 依赖项

- Tauri Dialog Plugin: `@tauri-apps/plugin-dialog`
- Tauri FS Plugin: `@tauri-apps/plugin-fs`
- Element Plus: 已有（用于按钮、表格、消息提示）

---

## 9. 设计决策记录

| 决策点 | 选择 | 理由 |
|--------|------|------|
| 交互方式 | 复选框模式 | 最直观，Element Plus 原生支持 |
| 按钮位置 | 工具栏 + 右键菜单 | 组合方式兼顾便利性和可见性 |
| 文件名 | 时间戳 + 保存对话框 | 默认有意义，允许自定义 |
| 数据获取 | 后端批量接口 | 性能最优，一次 IPC 调用 |
| 导出反馈 | 简单提示 | 批量接口性能好，无需复杂进度条 |

---

## 10. 未来扩展

- 支持按域名/状态码/时间范围过滤后导出
- 支持导出为其他格式（如 Postman Collection）
- 支持保存导出配置为预设
- 支持拖拽排序导出顺序
