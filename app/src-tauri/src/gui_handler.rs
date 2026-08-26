use async_trait::async_trait;
use mitm_core::{
    handler::HttpHandler,
    hyper::{Body, HeaderMap, Request, Response, StatusCode, header},
    mitm::{HttpContext, RequestOrResponse},
};
use rule::{Action, Rule, RuleHandlerCtx, RuleHttpHandler};
use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex, RwLock,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};
use tokio::sync::oneshot;

use crate::config::AppConfig;
use crate::intercept::{InterceptDecision, InterceptRequest};
use crate::traffic::{SharedTraffic, read_body};

/// 拦截事件转发回调（由上层注入，生产环境 emit 到前端，测试环境可捕获）。
type InterceptSink = Arc<dyn Fn(InterceptRequest) + Send + Sync>;
type TimeoutSink = Arc<dyn Fn(u64) + Send + Sync>;

/// 拦截决策等待超时：30 秒未响应默认放行。
const INTERCEPT_TIMEOUT: Duration = Duration::from_secs(30);

/// 包装 `RuleHttpHandler` 的流量捕获 + 交互拦截 handler：
/// - 委托内部规则引擎处理请求/响应；
/// - 在请求/响应阶段记录完整事务（headers + body）写入 `SharedTraffic`；
/// - 命中 `intercept` 规则时 emit 决策请求并阻塞等待用户回执。
#[derive(Clone)]
pub struct GuiHandler {
    inner: RuleHttpHandler,
    traffic: SharedTraffic,
    config: Arc<RwLock<AppConfig>>,
    /// 待决策的拦截请求表（id → oneshot Sender）。
    intercept_requests: Arc<Mutex<HashMap<u64, oneshot::Sender<InterceptDecision>>>>,
    intercept_id: Arc<AtomicU64>,
    emit_intercept: InterceptSink,
    emit_timeout: TimeoutSink,
}

impl GuiHandler {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        inner: RuleHttpHandler,
        traffic: SharedTraffic,
        config: Arc<RwLock<AppConfig>>,
        emit_intercept: InterceptSink,
        emit_timeout: TimeoutSink,
    ) -> Self {
        Self {
            inner,
            traffic,
            config,
            intercept_requests: Arc::new(Mutex::new(HashMap::new())),
            intercept_id: Arc::new(AtomicU64::new(1)),
            emit_intercept,
            emit_timeout,
        }
    }

    fn headers_vec(headers: &HeaderMap) -> Vec<(String, String)> {
        headers
            .iter()
            .map(|(k, v)| {
                (
                    k.as_str().to_string(),
                    v.to_str().unwrap_or_default().to_string(),
                )
            })
            .collect()
    }

    /// 热重载规则（转发给内部 `RuleHttpHandler`，改的是共享 `RwLock` 内数据）。
    pub fn set_rules(&self, rules: Vec<Rule>) {
        self.inner.set_rules(rules);
    }

    /// 判断请求是否命中含 `intercept` 动作的规则。
    fn matched_intercept(&self, req: &Request<Body>) -> bool {
        self.inner.get_rules().iter().any(|rule| {
            rule.actions.iter().any(|a| matches!(a, Action::Intercept))
                && rule.filters.iter().any(|f| f.is_match_req(req))
        })
    }

    /// 触发拦截：emit 决策请求，阻塞等待前端回执（30s 超时放行）。
    async fn await_intercept(&self, req: &Request<Body>) -> InterceptDecision {
        let id = self.intercept_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();
        self.intercept_requests.lock().unwrap().insert(id, tx);

        let payload = InterceptRequest {
            id,
            method: req.method().as_str().to_string(),
            url: req.uri().to_string(),
            headers: Self::headers_vec(req.headers()),
        };
        (self.emit_intercept)(payload);

        match tokio::time::timeout(INTERCEPT_TIMEOUT, rx).await {
            Ok(Ok(decision)) => {
                self.intercept_requests.lock().unwrap().remove(&id);
                decision
            }
            Ok(Err(_)) => {
                // Sender 被 drop（连接中断），清理并放行。
                self.intercept_requests.lock().unwrap().remove(&id);
                InterceptDecision::Allow
            }
            Err(_) => {
                // 超时，默认放行。
                self.intercept_requests.lock().unwrap().remove(&id);
                (self.emit_timeout)(id);
                InterceptDecision::Allow
            }
        }
    }

    /// 前端回传决策，唤醒阻塞的连接。
    pub fn resolve_intercept(&self, id: u64, decision: InterceptDecision) -> Result<(), String> {
        let sender = self.intercept_requests.lock().unwrap().remove(&id);
        match sender {
            Some(tx) => tx.send(decision).map_err(|_| "连接已关闭".to_string()),
            None => Err("未找到该拦截请求（可能已超时或已处理）".to_string()),
        }
    }

    fn reject_response() -> Response<Body> {
        Response::builder()
            .status(StatusCode::BAD_GATEWAY)
            .body(Body::default())
            .unwrap()
    }

    fn redirect_response(target: &str) -> Response<Body> {
        let mut res = Response::builder()
            .status(StatusCode::FOUND)
            .body(Body::default())
            .unwrap();
        if let Ok(v) = header::HeaderValue::from_str(target) {
            res.headers_mut().insert(header::LOCATION, v);
        }
        res
    }

    /// 用拦截响应完成一条事务记录并返回（请求体未捕获）。
    async fn finish_intercept(
        &self,
        id: u64,
        method: String,
        url: String,
        host: String,
        req_ct: Option<String>,
        res: Response<Body>,
    ) -> RequestOrResponse {
        let (parts, body) = res.into_parts();
        let res_ct = parts
            .headers
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(String::from);
        let res_headers = Self::headers_vec(&parts.headers);
        let config = self.config.read().unwrap().clone();
        let (body, res_body, res_size, truncated) =
            read_body(body, res_ct.as_deref(), &config).await;
        let status = parts.status.as_u16();
        let res = Response::from_parts(parts, body);

        self.traffic
            .begin_request(id, method, url, host, Vec::new(), None, 0, req_ct);
        self.traffic.complete(
            id,
            status,
            res_ct,
            res_headers,
            res_body,
            res_size,
            truncated,
        );
        RequestOrResponse::Response(res)
    }
}

#[async_trait]
impl HttpHandler<RuleHandlerCtx> for GuiHandler {
    async fn handle_request(
        &self,
        ctx: &mut HttpContext<RuleHandlerCtx>,
        req: Request<Body>,
    ) -> RequestOrResponse {
        let id = self.traffic.next_id();
        ctx.custom_data.request_id = id;

        let method = req.method().as_str().to_string();
        let url = req.uri().to_string();
        let host = req.uri().host().unwrap_or_default().to_string();
        let req_ct = req
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(String::from);

        // 交互拦截：命中 intercept 规则时先等待用户决策。
        if self.matched_intercept(&req) {
            match self.await_intercept(&req).await {
                InterceptDecision::Allow => {}
                InterceptDecision::Reject => {
                    return self
                        .finish_intercept(id, method, url, host, req_ct, Self::reject_response())
                        .await;
                }
                InterceptDecision::Redirect { url: target } => {
                    return self
                        .finish_intercept(
                            id,
                            method,
                            url,
                            host,
                            req_ct,
                            Self::redirect_response(&target),
                        )
                        .await;
                }
            }
        }

        // 委托内部规则引擎（reject/redirect/modify 等）。
        let result = self.inner.handle_request(ctx, req).await;

        match result {
            RequestOrResponse::Request(req) => {
                let (parts, body) = req.into_parts();
                let req_headers = Self::headers_vec(&parts.headers);
                let config = self.config.read().unwrap().clone();
                let (body, req_body, req_size, _truncated) =
                    read_body(body, req_ct.as_deref(), &config).await;
                let req = Request::from_parts(parts, body);

                self.traffic.begin_request(
                    id,
                    method,
                    url,
                    host,
                    req_headers,
                    req_body,
                    req_size,
                    req_ct,
                );
                RequestOrResponse::Request(req)
            }
            RequestOrResponse::Response(res) => {
                let (parts, body) = res.into_parts();
                let res_ct = parts
                    .headers
                    .get(header::CONTENT_TYPE)
                    .and_then(|v| v.to_str().ok())
                    .map(String::from);
                let res_headers = Self::headers_vec(&parts.headers);
                let config = self.config.read().unwrap().clone();
                let (body, res_body, res_size, truncated) =
                    read_body(body, res_ct.as_deref(), &config).await;
                let status = parts.status.as_u16();
                let res = Response::from_parts(parts, body);

                // 请求被拦截，不展示请求体。
                self.traffic
                    .begin_request(id, method, url, host, Vec::new(), None, 0, req_ct);
                self.traffic.complete(
                    id,
                    status,
                    res_ct,
                    res_headers,
                    res_body,
                    res_size,
                    truncated,
                );
                RequestOrResponse::Response(res)
            }
        }
    }

    async fn handle_response(
        &self,
        ctx: &mut HttpContext<RuleHandlerCtx>,
        res: Response<Body>,
    ) -> Response<Body> {
        let id = ctx.custom_data.request_id;

        let res = self.inner.handle_response(ctx, res).await;

        let (parts, body) = res.into_parts();
        let res_ct = parts
            .headers
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(String::from);
        let res_headers = Self::headers_vec(&parts.headers);
        let config = self.config.read().unwrap().clone();
        let (body, res_body, res_size, truncated) =
            read_body(body, res_ct.as_deref(), &config).await;
        let status = parts.status.as_u16();
        let res = Response::from_parts(parts, body);

        if id != 0 {
            self.traffic.complete(
                id,
                status,
                res_ct,
                res_headers,
                res_body,
                res_size,
                truncated,
            );
        }
        res
    }
}
