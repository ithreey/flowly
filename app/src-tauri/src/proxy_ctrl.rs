use tokio::sync::oneshot;

use crate::state::AppState;
use crate::system_proxy::{self, SystemProxyGuard};

/// 正在运行的代理句柄。
pub struct ProxyHandle {
    pub shutdown_tx: Option<oneshot::Sender<()>>,
    pub join_handle: tokio::task::JoinHandle<()>,
    pub listen_addr: String,
    pub upstream_proxy: Option<String>,
    /// 自动设置系统代理时保存的还原 guard。
    pub system_proxy: Option<SystemProxyGuard>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyStatus {
    pub running: bool,
    pub listen_addr: String,
    pub upstream_proxy: Option<String>,
}

impl Default for ProxyStatus {
    fn default() -> Self {
        Self {
            running: false,
            listen_addr: String::new(),
            upstream_proxy: None,
        }
    }
}

/// 将 oneshot 接收端包装成 `Proxy.shutdown_signal` 需要的 future。
async fn oneshot_shutdown(rx: oneshot::Receiver<()>) {
    let _ = rx.await;
}

#[tauri::command]
pub async fn proxy_start(
    state: tauri::State<'_, AppState>,
    listen_addr: String,
    upstream_proxy: Option<String>,
) -> Result<ProxyStatus, String> {
    // 已运行则拒绝重复启动。
    if state.proxy.lock().unwrap().is_some() {
        return Err("代理已在运行，请先停止".to_string());
    }

    let addr: std::net::SocketAddr = listen_addr
        .parse()
        .map_err(|e| format!("监听地址无效: {e}"))?;

    // 预检端口可用性（Proxy 内部 bind 失败会异步返回，这里先同步拦截）。
    let _probe = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| format!("端口监听失败: {e}"))?;
    drop(_probe);

    // 可选上游代理（明文 HTTP 代理）。
    let upstream = match upstream_proxy.clone() {
        Some(p) => {
            let uri: mitm_core::hyper::Uri =
                p.parse().map_err(|e| format!("上游代理地址无效: {e}"))?;
            Some(flowly::hyper_proxy::Proxy::new(
                flowly::hyper_proxy::Intercept::All,
                uri,
            ))
        }
        None => None,
    };

    // 自动设置系统代理（受 config.auto_system_proxy 控制）。
    let auto_system_proxy = state.config.read().unwrap().auto_system_proxy;
    let system_proxy = if auto_system_proxy {
        Some(system_proxy::set_system_proxy(&listen_addr).map_err(|e| {
            format!("自动设置系统代理失败，代理未启动：{e}（可在设置中关闭该选项）")
        })?)
    } else {
        None
    };

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    let proxy = mitm_core::Proxy::builder()
        .ca(state.ca.lock().unwrap().clone())
        .listen_addr(addr)
        .upstream_proxy(upstream)
        .shutdown_signal(oneshot_shutdown(shutdown_rx))
        .mitm_filter(state.mitm_filter.clone())
        .handler(state.handler.clone())
        .build();

    let join_handle = tokio::spawn(async move {
        if let Err(e) = proxy.start_proxy().await {
            eprintln!("代理启动失败: {e}");
        }
    });

    let status = ProxyStatus {
        running: true,
        listen_addr: listen_addr.clone(),
        upstream_proxy: upstream_proxy.clone(),
    };
    *state.proxy.lock().unwrap() = Some(ProxyHandle {
        shutdown_tx: Some(shutdown_tx),
        join_handle,
        listen_addr,
        upstream_proxy,
        system_proxy,
    });

    Ok(status)
}

#[tauri::command]
pub async fn proxy_stop(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let handle = state.proxy.lock().unwrap().take();
    if let Some(handle) = handle {
        if let Some(tx) = handle.shutdown_tx {
            let _ = tx.send(());
        }
        // 还原系统代理（若有设置）。
        if let Some(guard) = &handle.system_proxy {
            system_proxy::restore_system_proxy(guard);
        }
        // 等待 accept 循环退出（含 2s 超时兜底，避免异常时挂起）。
        let _ = tokio::time::timeout(std::time::Duration::from_secs(2), handle.join_handle).await;
    }
    Ok(())
}

#[tauri::command]
pub async fn proxy_status(state: tauri::State<'_, AppState>) -> Result<ProxyStatus, String> {
    let guard = state.proxy.lock().unwrap();
    Ok(match guard.as_ref() {
        Some(handle) => ProxyStatus {
            running: true,
            listen_addr: handle.listen_addr.clone(),
            upstream_proxy: handle.upstream_proxy.clone(),
        },
        None => ProxyStatus::default(),
    })
}

/// 停止代理并还原系统代理（同步版，供程序退出时调用）。
pub fn shutdown_sync(state: &AppState) {
    let handle = state.proxy.lock().unwrap().take();
    if let Some(handle) = handle {
        if let Some(tx) = handle.shutdown_tx {
            let _ = tx.send(());
        }
        if let Some(guard) = &handle.system_proxy {
            system_proxy::restore_system_proxy(guard);
        }
    }
}
