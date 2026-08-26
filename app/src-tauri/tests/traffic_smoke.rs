//! 流量捕获与规则热重载冒烟测试：
//! - `capture_http_traffic`：走代理请求本地 HTTP 服务，验证请求/响应被捕获。
//! - `hot_reload_rules`：不重启代理，热重载规则后立即生效（reject → 502）。

use flowly::hyper_proxy::{Intercept, Proxy, ProxyConnector};
use flowly_gui::config::AppConfig;
use flowly_gui::gui_handler::GuiHandler;
use flowly_gui::traffic::SharedTraffic;
use mitm_core::handler::MitmFilter;
use mitm_core::hyper::{Client, Uri, body::to_bytes, client::HttpConnector};
use rule::{Rule, RuleHandlerCtx, RuleHttpHandler};
use std::sync::{Arc, RwLock};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

type ProxyClient = Client<ProxyConnector<HttpConnector>, mitm_core::hyper::Body>;

/// 构造一个可用的 CertificateAuthority（gen_ca → PEM → rustls）。
fn build_ca() -> mitm_core::CertificateAuthority {
    let cert = mitm_core::CertificateAuthority::gen_ca().expect("gen_ca");
    let cert_pem = cert.serialize_pem().unwrap();
    let key_pem = cert.serialize_private_key_pem();

    let mut key_bytes: &[u8] = key_pem.as_bytes();
    let private_key = rustls_pemfile::pkcs8_private_keys(&mut key_bytes).unwrap();
    let private_key = rustls::PrivateKey(private_key[0].clone());

    let mut cert_bytes: &[u8] = cert_pem.as_bytes();
    let ca_cert = rustls_pemfile::certs(&mut cert_bytes).unwrap();
    let ca_cert = rustls::Certificate(ca_cert[0].clone());

    mitm_core::CertificateAuthority::new(private_key, ca_cert, cert_pem, 100).unwrap()
}

/// 起一个本地 HTTP 服务，返回固定响应 "hello"。
async fn spawn_http_server() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                break;
            };
            tokio::spawn(async move {
                let mut buf = [0u8; 4096];
                let _ = stream.read(&mut buf).await;
                let resp = "HTTP/1.1 200 OK\r\ncontent-type: text/plain\r\ncontent-length: 5\r\nconnection: close\r\n\r\nhello";
                let _ = stream.write_all(resp.as_bytes()).await;
            });
        }
    });
    port
}

/// 找一个空闲端口（先 bind :0 再释放，供 Proxy 使用）。
async fn free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    listener.local_addr().unwrap().port()
}

/// 构造拦截事件回调（测试中拦截分支不触发，给空实现）。
fn mock_intercept_sinks() -> (
    std::sync::Arc<dyn Fn(flowly_gui::intercept::InterceptRequest) + Send + Sync>,
    std::sync::Arc<dyn Fn(u64) + Send + Sync>,
) {
    (
        std::sync::Arc::new(|_req| {}),
        std::sync::Arc::new(|_id| {}),
    )
}

/// 起一个真实代理，返回 (监听端口, shutdown_tx, join_handle)。
async fn spawn_proxy(
    handler: GuiHandler,
    mitm_filter: Arc<MitmFilter<RuleHandlerCtx>>,
) -> (
    u16,
    tokio::sync::oneshot::Sender<()>,
    tokio::task::JoinHandle<()>,
) {
    let port = free_port().await;
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let shutdown = async move {
        let _ = shutdown_rx.await;
    };
    let proxy = mitm_core::Proxy::builder()
        .ca(build_ca())
        .listen_addr(format!("127.0.0.1:{port}").parse().unwrap())
        .upstream_proxy(None)
        .shutdown_signal(shutdown)
        .mitm_filter(mitm_filter)
        .handler(handler)
        .build();
    let join = tokio::spawn(async move {
        let _ = proxy.start_proxy().await;
    });
    (port, shutdown_tx, join)
}

/// 走 `proxy_port` 代理的 HTTP client。
fn proxy_client(proxy_port: u16) -> ProxyClient {
    let uri: Uri = format!("http://127.0.0.1:{proxy_port}").parse().unwrap();
    let upstream = Proxy::new(Intercept::All, uri);
    let connector = ProxyConnector::from_proxy_unsecured(HttpConnector::new(), upstream);
    Client::builder().build(connector)
}

#[tokio::test]
async fn capture_http_traffic() {
    let server_port = spawn_http_server().await;

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

    let target = format!("http://127.0.0.1:{server_port}/hello");
    let res = client
        .get(target.parse().unwrap())
        .await
        .expect("proxy request");
    let body = to_bytes(res.into_body()).await.unwrap();
    assert_eq!(body.as_ref(), b"hello", "响应体应为 hello");

    // 等待捕获写入（broadcast 异步，稍等）。
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let list = traffic.list(100, 0);
    assert_eq!(list.len(), 1, "应捕获 1 条事务");
    let summary = &list[0];
    assert_eq!(summary.method, "GET");
    assert_eq!(summary.status, Some(200));
    assert!(summary.url.contains("/hello"));
    assert_eq!(summary.res_size, 5);

    // 完整事务详情含响应体。
    let detail = traffic.get(summary.id).expect("detail");
    assert_eq!(detail.res_body.as_deref(), Some("hello"));
    assert!(
        detail
            .res_headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("content-type"))
    );

    let _ = shutdown_tx.send(());
    let _ = tokio::time::timeout(std::time::Duration::from_secs(2), proxy_task).await;
}

#[tokio::test]
async fn hot_reload_rules() {
    let server_port = spawn_http_server().await;

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
        spawn_proxy(handler.clone(), Arc::clone(&mitm_filter)).await;
    let client = proxy_client(proxy_port);
    let target = format!("http://127.0.0.1:{server_port}/test");

    // 第一次：无规则 → 200。
    let res = client.get(target.parse().unwrap()).await.unwrap();
    assert_eq!(res.status(), 200, "无规则时应正常转发");

    // 热重载：加入 reject 规则（不重启代理）。
    let reject_rule = Rule {
        filters: vec![rule::Filter::DomainKeyword("127.0.0.1".to_string())],
        actions: vec![rule::Action::Reject],
        url: None,
    };
    handler.set_rules(vec![reject_rule]);
    mitm_filter.set_filters(vec![]);

    // 第二次：被 reject → 502（证明规则热生效）。
    let res = client.get(target.parse().unwrap()).await.unwrap();
    assert_eq!(res.status(), 502, "热重载后应被 reject");

    let _ = shutdown_tx.send(());
    let _ = tokio::time::timeout(std::time::Duration::from_secs(2), proxy_task).await;
}
