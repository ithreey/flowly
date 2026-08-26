//! 交互拦截：命中 `intercept` 规则的请求触发前端决策窗口。

use serde::{Deserialize, Serialize};

use crate::state::AppState;

/// 前端决策结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum InterceptDecision {
    /// 放行，正常转发。
    Allow,
    /// 拒绝，返回 502。
    Reject,
    /// 重定向到指定 URL，返回 302。
    Redirect { url: String },
}

/// emit 给前端的待决策请求摘要。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InterceptRequest {
    pub id: u64,
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
}

/// 前端回传决策，唤醒阻塞的连接。
#[tauri::command]
pub fn intercept_decide(
    state: tauri::State<'_, AppState>,
    id: u64,
    decision: InterceptDecision,
) -> Result<(), String> {
    state.handler.resolve_intercept(id, decision)
}
