use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

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

fn spawn_proxy_join_cleanup(join_handle: tokio::task::JoinHandle<()>) {
    tokio::spawn(async move {
        let _ = tokio::time::timeout(std::time::Duration::from_secs(2), join_handle).await;
    });
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

    let addr = normalize_listen_addr(&listen_addr)?;
    let listen_addr = addr.to_string();

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
        let system_proxy_addr = system_proxy_addr(addr);
        Some(system_proxy::set_system_proxy(&system_proxy_addr).map_err(|e| {
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
        // 后台等待 accept 循环退出，避免停止按钮被 2s 兜底等待阻塞。
        spawn_proxy_join_cleanup(handle.join_handle);
    }
    Ok(())
}

pub(crate) fn system_proxy_addr(listen_addr: SocketAddr) -> String {
    if listen_addr.ip().is_unspecified() {
        let local_ip = match listen_addr.ip() {
            IpAddr::V4(_) => IpAddr::V4(Ipv4Addr::LOCALHOST),
            IpAddr::V6(_) => IpAddr::V6(Ipv6Addr::LOCALHOST),
        };
        return SocketAddr::new(local_ip, listen_addr.port()).to_string();
    }

    listen_addr.to_string()
}

pub(crate) fn listen_addr_from_port(port: u16) -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port)
}

pub(crate) fn normalize_listen_addr(value: &str) -> Result<SocketAddr, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("监听端口不能为空".to_string());
    }

    if !value.contains(':') {
        let port = value
            .parse::<u16>()
            .map_err(|e| format!("监听端口无效: {e}"))?;
        return Ok(listen_addr_from_port(port));
    }

    value.parse().map_err(|e| format!("监听地址无效: {e}"))
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

#[cfg(test)]
mod tests {
    use super::{
        listen_addr_from_port, normalize_listen_addr, spawn_proxy_join_cleanup, system_proxy_addr,
    };
    use std::time::{Duration, Instant};

    #[test]
    fn system_proxy_addr_uses_loopback_for_unspecified_ipv4_listen_addr() {
        let listen_addr = "0.0.0.0:34567".parse().unwrap();

        assert_eq!(system_proxy_addr(listen_addr), "127.0.0.1:34567");
    }

    #[test]
    fn system_proxy_addr_preserves_specific_listen_addr() {
        let listen_addr = "192.168.1.10:34567".parse().unwrap();

        assert_eq!(system_proxy_addr(listen_addr), "192.168.1.10:34567");
    }

    #[test]
    fn listen_addr_from_port_listens_on_all_ipv4_interfaces() {
        assert_eq!(listen_addr_from_port(34567).to_string(), "0.0.0.0:34567");
    }

    #[test]
    fn normalize_listen_addr_accepts_port_only() {
        assert_eq!(normalize_listen_addr("34567").unwrap().to_string(), "0.0.0.0:34567");
    }

    #[test]
    fn normalize_listen_addr_preserves_legacy_full_addr() {
        assert_eq!(
            normalize_listen_addr("127.0.0.1:34567").unwrap().to_string(),
            "127.0.0.1:34567"
        );
    }

    #[tokio::test]
    async fn spawn_proxy_join_cleanup_does_not_wait_for_join_handle_to_finish() {
        let join_handle = tokio::spawn(async {
            tokio::time::sleep(Duration::from_secs(5)).await;
        });

        let started = Instant::now();
        spawn_proxy_join_cleanup(join_handle);

        assert!(
            started.elapsed() < Duration::from_millis(100),
            "cleanup should be detached from the caller"
        );
    }
}
