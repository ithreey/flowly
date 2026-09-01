use mitm_core::hyper;
use mitm_core::hyper::body::HttpBody;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, VecDeque},
    sync::{
        Arc, RwLock,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, SystemTime},
};
use tauri::Emitter;
use tokio::sync::broadcast;

use crate::config::AppConfig;

/// 摘要环容量：列表页只保留最近 500 条事务摘要（不含 body）。
const SUMMARY_RING_CAP: usize = 500;
/// broadcast 通道容量。
const CHANNEL_CAPACITY: usize = 1024;

/// 单条事务的摘要信息，推送给前端列表展示。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficSummary {
    pub id: u64,
    pub method: String,
    pub url: String,
    pub host: String,
    pub phase: TrafficPhase,
    pub status: Option<u16>,
    pub content_type: Option<String>,
    pub req_size: usize,
    pub res_size: usize,
    pub duration_ms: u128,
    /// 请求开始时间（unix 毫秒）。
    pub started_at: u128,
    /// body 超过上限被截断，或请求阶段被短路时标记。
    pub truncated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TrafficPhase {
    Pending,
    Completed,
    Failed,
}

/// 完整事务（含请求/响应 headers 与 body），按 id 拉取。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionDetail {
    pub summary: TrafficSummary,
    pub req_headers: Vec<(String, String)>,
    pub req_body: Option<String>,
    pub res_headers: Vec<(String, String)>,
    pub res_body: Option<String>,
}

/// 轻量事务详情，不含请求/响应 body，用于详情抽屉首屏快速渲染。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionMeta {
    pub summary: TrafficSummary,
    pub req_headers: Vec<(String, String)>,
    pub res_headers: Vec<(String, String)>,
    pub req_body_available: bool,
    pub res_body_available: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TrafficBodyKind {
    Request,
    Response,
}

/// 全局共享的流量记录：
/// - `summaries`：摘要环（分页查询）
/// - `transactions`：完整事务表（含 body），生命周期由摘要环统一控制
/// - `tx`：broadcast，实时推送给前端
#[derive(Clone)]
pub struct SharedTraffic {
    next_id: Arc<AtomicU64>,
    tx: broadcast::Sender<TrafficSummary>,
    summaries: Arc<RwLock<VecDeque<TrafficSummary>>>,
    transactions: Arc<RwLock<HashMap<u64, TransactionDetail>>>,
}

impl Default for SharedTraffic {
    fn default() -> Self {
        Self::new()
    }
}

impl SharedTraffic {
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(CHANNEL_CAPACITY);
        Self {
            next_id: Arc::new(AtomicU64::new(1)),
            tx,
            summaries: Arc::new(RwLock::new(VecDeque::with_capacity(SUMMARY_RING_CAP))),
            transactions: Arc::new(RwLock::new(HashMap::with_capacity(SUMMARY_RING_CAP))),
        }
    }

    pub fn next_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    pub fn subscribe(&self) -> broadcast::Receiver<TrafficSummary> {
        self.tx.subscribe()
    }

    /// 请求阶段：登记一条事务，等响应阶段 `complete`。
    pub fn begin_request(
        &self,
        id: u64,
        method: String,
        url: String,
        host: String,
        req_headers: Vec<(String, String)>,
        req_body: Option<String>,
        req_size: usize,
        content_type: Option<String>,
    ) {
        let summary = TrafficSummary {
            id,
            method,
            url,
            host,
            phase: TrafficPhase::Pending,
            status: None,
            content_type,
            req_size,
            res_size: 0,
            duration_ms: 0,
            started_at: now_unix_millis(),
            truncated: false,
        };
        self.transactions.write().unwrap().insert(
            id,
            TransactionDetail {
                summary: summary.clone(),
                req_headers,
                req_body,
                res_headers: Vec::new(),
                res_body: None,
            },
        );
        self.upsert_summary(summary);
    }

    /// 响应阶段：补全事务，写入摘要环并广播。
    pub fn complete(
        &self,
        id: u64,
        status: u16,
        content_type: Option<String>,
        res_headers: Vec<(String, String)>,
        res_body: Option<String>,
        res_size: usize,
        truncated: bool,
    ) {
        let summary = {
            let mut transactions = self.transactions.write().unwrap();
            let Some(detail) = transactions.get_mut(&id) else {
                return;
            };
            let start = detail.summary.started_at;
            let now = now_unix_millis();

            detail.summary.status = Some(status);
            detail.summary.phase = TrafficPhase::Completed;
            detail.summary.content_type =
                content_type.or_else(|| detail.summary.content_type.take());
            detail.summary.res_size = res_size;
            detail.summary.duration_ms = now.saturating_sub(start);
            detail.summary.truncated = truncated;
            detail.res_headers = res_headers;
            detail.res_body = res_body;

            detail.summary.clone()
        };
        self.upsert_summary(summary);
    }

    pub fn fail(&self, id: u64) {
        let summary = {
            let mut transactions = self.transactions.write().unwrap();
            let Some(detail) = transactions.get_mut(&id) else {
                return;
            };
            let start = detail.summary.started_at;
            let now = now_unix_millis();

            detail.summary.phase = TrafficPhase::Failed;
            detail.summary.duration_ms = now.saturating_sub(start);

            detail.summary.clone()
        };
        self.upsert_summary(summary);
    }

    fn upsert_summary(&self, summary: TrafficSummary) {
        let mut ring = self.summaries.write().unwrap();
        if let Some(existing) = ring.iter_mut().find(|item| item.id == summary.id) {
            *existing = summary.clone();
        } else if ring.len() >= SUMMARY_RING_CAP {
            if let Some(expired) = ring.pop_front() {
                self.transactions.write().unwrap().remove(&expired.id);
            }
            ring.push_back(summary.clone());
        } else {
            ring.push_back(summary.clone());
        }
        drop(ring);

        let _ = self.tx.send(summary);
    }

    /// 分页读取摘要（旧数据重载用，不含 body）。
    pub fn list(&self, limit: usize, offset: usize) -> Vec<TrafficSummary> {
        let ring = self.summaries.read().unwrap();
        ring.iter().skip(offset).take(limit).cloned().collect()
    }

    /// 按 id 拉取完整事务（含 headers/body）。
    pub fn get(&self, id: u64) -> Option<TransactionDetail> {
        self.transactions.read().unwrap().get(&id).cloned()
    }

    /// 按 id 拉取轻量事务详情，不克隆 body 文本。
    pub fn get_meta(&self, id: u64) -> Option<TransactionMeta> {
        self.transactions
            .read()
            .unwrap()
            .get(&id)
            .map(|detail| TransactionMeta {
                summary: detail.summary.clone(),
                req_headers: detail.req_headers.clone(),
                res_headers: detail.res_headers.clone(),
                req_body_available: detail.req_body.is_some(),
                res_body_available: detail.res_body.is_some(),
            })
    }

    /// 按 id 与类型拉取单侧 body，避免打开详情时一次性传输大文本。
    pub fn get_body(&self, id: u64, kind: TrafficBodyKind) -> Option<String> {
        let transactions = self.transactions.read().unwrap();
        let detail = transactions.get(&id)?;
        match kind {
            TrafficBodyKind::Request => detail.req_body.clone(),
            TrafficBodyKind::Response => detail.res_body.clone(),
        }
    }

    /// 批量获取完整事务（按 id 列表），保持顺序，缺失的条目为 None。
    pub fn get_batch(&self, ids: &[u64]) -> Vec<Option<TransactionDetail>> {
        let transactions = self.transactions.read().unwrap();
        ids.iter().map(|id| transactions.get(id).cloned()).collect()
    }

    pub fn clear(&self) {
        self.summaries.write().unwrap().clear();
        self.transactions.write().unwrap().clear();
    }

    pub fn delete(&self, ids: &[u64]) {
        if ids.is_empty() {
            return;
        }
        let mut ring = self.summaries.write().unwrap();
        ring.retain(|summary| !ids.contains(&summary.id));
        drop(ring);

        let mut transactions = self.transactions.write().unwrap();
        for id in ids {
            transactions.remove(id);
        }
    }
}

fn now_unix_millis() -> u128 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

/// 启动 broadcast → 前端事件桥接任务：
/// 摘要攒批（每 50ms 或攒满 50 条）emit 一次 `traffic://batch`，降低事件吞吐压力。
pub fn spawn_traffic_bridge(app: tauri::AppHandle, mut rx: broadcast::Receiver<TrafficSummary>) {
    tauri::async_runtime::spawn(async move {
        let mut batch: Vec<TrafficSummary> = Vec::new();
        let mut interval = tokio::time::interval(Duration::from_millis(50));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if !batch.is_empty() {
                        let _ = app.emit("traffic://batch", std::mem::take(&mut batch));
                    }
                }
                item = rx.recv() => {
                    match item {
                        Ok(summary) => {
                            batch.push(summary);
                            if batch.len() >= 50 {
                                let _ = app.emit("traffic://batch", std::mem::take(&mut batch));
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => {}
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
            }
        }
    });
}

/// 判断给定的 Content-Type 是否值得捕获 body（文本类/form，且排除 SSE 流）。
///
/// 复用 `rule` crate 的 `is_textual_content_type` 语义，额外排除
/// `text/event-stream`（SSE 流式响应会因全量缓冲而被破坏）。
pub fn should_capture_content_type(content_type: Option<&str>) -> bool {
    let Some(ct) = content_type else {
        return false;
    };
    let mime = ct
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    if mime == "text/event-stream" {
        return false;
    }
    if mime == "application/x-www-form-urlencoded" {
        return true;
    }
    let Ok(header_value) = hyper::http::HeaderValue::from_str(ct) else {
        return false;
    };
    rule::is_textual_content_type(&header_value)
}

/// 按配置读取请求/响应 body 并重建。返回 (重建后的 body, 捕获文本, 实际大小, 是否截断)。
///
/// - 未开启捕获或非文本类型：不读 body，返回原 body（大小用 size_hint 估计）。
/// - 超过 `max_body_size`：置 `truncated=true`，不保留 body 内容。
pub async fn read_body(
    body: hyper::Body,
    content_type: Option<&str>,
    config: &AppConfig,
) -> (hyper::Body, Option<String>, usize, bool) {
    if !config.capture_body || !should_capture_content_type(content_type) {
        let size = body.size_hint().lower() as usize;
        return (body, None, size, false);
    }

    match hyper::body::to_bytes(body).await {
        Ok(bytes) => {
            let size = bytes.len();
            if size > config.max_body_size {
                return (hyper::Body::from(bytes), None, size, true);
            }
            let text = String::from_utf8_lossy(&bytes).into_owned();
            (hyper::Body::from(bytes), Some(text), size, false)
        }
        Err(_) => (hyper::Body::empty(), None, 0, false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn begin_request_lists_pending_and_complete_updates_same_summary() {
        let traffic = SharedTraffic::new();
        let id = traffic.next_id();

        traffic.begin_request(
            id,
            "GET".to_string(),
            "http://example.test/path".to_string(),
            "example.test".to_string(),
            Vec::new(),
            None,
            0,
            None,
        );

        let list = traffic.list(100, 0);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, id);
        assert_eq!(list[0].phase, TrafficPhase::Pending);
        assert_eq!(list[0].status, None);

        traffic.complete(
            id,
            200,
            Some("text/plain".to_string()),
            Vec::new(),
            Some("ok".to_string()),
            2,
            false,
        );

        let list = traffic.list(100, 0);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, id);
        assert_eq!(list[0].phase, TrafficPhase::Completed);
        assert_eq!(list[0].status, Some(200));
        assert_eq!(list[0].res_size, 2);
    }

    #[test]
    fn get_meta_returns_headers_and_body_availability_without_body_text() {
        let traffic = SharedTraffic::new();
        traffic.begin_request(
            1,
            "POST".to_string(),
            "http://example.test/path".to_string(),
            "example.test".to_string(),
            vec![("content-type".to_string(), "text/plain".to_string())],
            Some("request body".to_string()),
            12,
            Some("text/plain".to_string()),
        );
        traffic.complete(
            1,
            200,
            Some("application/json".to_string()),
            vec![("content-type".to_string(), "application/json".to_string())],
            Some("{\"ok\":true}".to_string()),
            11,
            false,
        );

        let meta = traffic.get_meta(1).expect("meta");

        assert_eq!(meta.summary.method, "POST");
        assert_eq!(meta.req_headers.len(), 1);
        assert_eq!(meta.res_headers.len(), 1);
        assert!(meta.req_body_available);
        assert!(meta.res_body_available);
    }

    #[test]
    fn get_body_returns_request_or_response_body_only() {
        let traffic = SharedTraffic::new();
        traffic.begin_request(
            1,
            "POST".to_string(),
            "http://example.test/path".to_string(),
            "example.test".to_string(),
            Vec::new(),
            Some("request body".to_string()),
            12,
            None,
        );
        traffic.complete(
            1,
            200,
            None,
            Vec::new(),
            Some("response body".to_string()),
            13,
            false,
        );

        assert_eq!(
            traffic.get_body(1, TrafficBodyKind::Request),
            Some("request body".to_string())
        );
        assert_eq!(
            traffic.get_body(1, TrafficBodyKind::Response),
            Some("response body".to_string())
        );
    }
}
