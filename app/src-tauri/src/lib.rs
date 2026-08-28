pub mod cert_cmd;
pub mod config;
pub mod gui_handler;
pub mod history;
pub mod intercept;
mod proxy_ctrl;
mod rules_cmd;
mod state;
pub mod system_proxy;
pub mod traffic;

use std::sync::{Arc, RwLock};

use mitm_core::{
    handler::MitmFilter,
    http_client::{HttpClient, gen_client},
    hyper::{Body, Request, Uri, body::HttpBody, body::to_bytes, header},
};
use rule::{RuleHandlerCtx, RuleHttpHandler};
use tauri::{Emitter, Manager, PhysicalPosition, PhysicalSize, Position, Size, WindowEvent};

use crate::config::AppConfig;
use crate::gui_handler::GuiHandler;
use crate::intercept::InterceptRequest;
use crate::state::AppState;
use crate::traffic::{SharedTraffic, TransactionDetail, spawn_traffic_bridge};

fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    fit_main_window_to_screen(app);

    let data_dir = state::app_data_dir(app)?;
    let rules_path = data_dir.join("rules.json");
    let ca_path = data_dir.join("ca");
    let config_path = data_dir.join("config.json");
    let history_path = data_dir.join("history.json");

    // 从 config.json 加载配置（文件不存在时用默认值）。
    let has_global_mitm_config = config::has_mitm_hosts_config(&config_path);
    let config: Arc<RwLock<AppConfig>> = Arc::new(RwLock::new(config::load_config(&config_path)));
    let ca = state::ensure_ca(&data_dir)?;
    let (rules, legacy_mitm_filters) = state::ensure_rules(&data_dir)?;
    let mitm_filters = {
        let cfg = config.read().unwrap();
        if has_global_mitm_config {
            cfg.mitm_hosts.clone()
        } else {
            legacy_mitm_filters
        }
    };
    if let Err(e) = system_proxy::clear_stale_system_proxy(&config.read().unwrap().listen_addr) {
        eprintln!("清理遗留系统代理失败: {e}");
    }
    let traffic = SharedTraffic::new();

    // 拦截事件转发到前端。
    let app_handle = app.handle().clone();
    let emit_intercept: std::sync::Arc<dyn Fn(InterceptRequest) + Send + Sync> =
        std::sync::Arc::new(move |req| {
            let _ = app_handle.emit("intercept://pending", req);
        });
    let app_handle = app.handle().clone();
    let emit_timeout: std::sync::Arc<dyn Fn(u64) + Send + Sync> = std::sync::Arc::new(move |id| {
        let _ = app_handle.emit("intercept://timeout", id);
    });

    let handler = GuiHandler::new(
        RuleHttpHandler::new(rules.clone()),
        traffic.clone(),
        config.clone(),
        emit_intercept,
        emit_timeout,
    );
    let mitm_filter: Arc<MitmFilter<RuleHandlerCtx>> = Arc::new(MitmFilter::new(mitm_filters));

    // broadcast → 前端事件桥接。
    spawn_traffic_bridge(app.handle().clone(), traffic.subscribe());

    let history = history::HistoryStore::new(&history_path);

    app.manage(AppState {
        proxy: std::sync::Mutex::new(None),
        ca: std::sync::Mutex::new(ca),
        rules,
        mitm_filter,
        handler,
        config,
        traffic,
        rules_path,
        ca_path,
        config_path,
        history_path,
        history,
    });

    Ok(())
}

fn fit_main_window_to_screen(app: &tauri::App) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let Ok(Some(monitor)) = window.current_monitor() else {
        return;
    };

    let work_area = monitor.work_area();
    let Ok(outer_size) = window.outer_size() else {
        return;
    };

    let target_height = (f64::from(work_area.size.height) * 0.8).round() as u32;
    let target_width = outer_size.width.min(work_area.size.width);
    let target_x =
        work_area.position.x + ((work_area.size.width.saturating_sub(target_width)) / 2) as i32;
    let target_y =
        work_area.position.y + ((work_area.size.height.saturating_sub(target_height)) / 2) as i32;

    if let Err(e) = window.set_size(Size::Physical(PhysicalSize::new(
        target_width,
        target_height,
    ))) {
        eprintln!("设置主窗口默认尺寸失败: {e}");
    }
    if let Err(e) = window.set_position(Position::Physical(PhysicalPosition::new(
        target_x, target_y,
    ))) {
        eprintln!("设置主窗口默认位置失败: {e}");
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(setup)
        .on_window_event(|window, event| {
            if matches!(event, WindowEvent::CloseRequested { .. }) {
                let state = window.state::<AppState>();
                proxy_ctrl::shutdown_sync(&state);
            }
        })
        .invoke_handler(tauri::generate_handler![
            proxy_ctrl::proxy_start,
            proxy_ctrl::proxy_stop,
            proxy_ctrl::proxy_status,
            config_get,
            config_set,
            traffic_get,
            traffic_get_batch,
            traffic_replay,
            traffic_clear,
            traffic_delete,
            rules_cmd::rules_list,
            rules_cmd::rules_save,
            rules_cmd::rules_import_json,
            cert_cmd::cert_status,
            cert_cmd::cert_generate,
            cert_cmd::cert_get_pem,
            cert_cmd::cert_install_trust,
            intercept::intercept_decide,
            history::history_list,
            history::history_save,
            history::history_clear,
            history::history_delete,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn config_get(state: tauri::State<'_, AppState>) -> Result<AppConfig, String> {
    Ok(state.config.read().unwrap().clone())
}

#[tauri::command]
fn config_set(state: tauri::State<'_, AppState>, config: AppConfig) -> Result<(), String> {
    *state.config.write().unwrap() = config.clone();
    config::save_config(&state.config_path, &config)
}

#[tauri::command]
fn traffic_get(
    state: tauri::State<'_, AppState>,
    id: u64,
) -> Result<Option<TransactionDetail>, String> {
    Ok(state.traffic.get(id))
}

#[tauri::command]
fn traffic_get_batch(
    state: tauri::State<'_, AppState>,
    ids: Vec<u64>,
) -> Result<Vec<Option<TransactionDetail>>, String> {
    Ok(state.traffic.get_batch(&ids))
}

#[tauri::command]
async fn traffic_replay(state: tauri::State<'_, AppState>, id: u64) -> Result<u64, String> {
    replay_traffic_request(state.traffic.clone(), id).await
}

pub async fn replay_traffic_request(traffic: SharedTraffic, id: u64) -> Result<u64, String> {
    let detail = traffic
        .get(id)
        .ok_or_else(|| "会话已过期或已删除，无法重放".to_string())?;
    let replay_id = traffic.next_id();
    let method = detail.summary.method.clone();
    let url = detail.summary.url.clone();
    let host = detail.summary.host.clone();
    let req_headers = sanitize_replay_headers(&detail.req_headers);
    let req_body = detail.req_body.clone();
    let request_body = req_body.clone().unwrap_or_default();
    let req_ct = req_headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
        .map(|(_, value)| value.clone());

    let uri: Uri = url.parse().map_err(|e| format!("URL 无效，无法重放：{e}"))?;
    let mut builder = Request::builder().method(method.as_str()).uri(uri);
    for (name, value) in &req_headers {
        builder = builder.header(name.as_str(), value.as_str());
    }
    let request = builder
        .body(Body::from(request_body))
        .map_err(|e| format!("构造重放请求失败：{e}"))?;
    let client = gen_client(None).map_err(|e| format!("创建重放客户端失败：{e}"))?;

    traffic.begin_request(
        replay_id,
        method.clone(),
        url.clone(),
        host,
        req_headers.clone(),
        req_body,
        request.body().size_hint().lower() as usize,
        req_ct,
    );

    let response = match client {
        HttpClient::Https(client) => match client.request(request).await {
            Ok(response) => response,
            Err(e) => {
                traffic.fail(replay_id);
                return Err(format!("重放请求失败：{e}"));
            }
        },
        HttpClient::Proxy(_) => return Err("重放客户端初始化异常".to_string()),
    };

    let (parts, body) = response.into_parts();
    let res_ct = parts
        .headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(String::from);
    let res_headers = parts
        .headers
        .iter()
        .map(|(k, v)| {
            (
                k.as_str().to_string(),
                v.to_str().unwrap_or_default().to_string(),
            )
        })
        .collect();
    let bytes = match to_bytes(body).await {
        Ok(bytes) => bytes,
        Err(e) => {
            traffic.fail(replay_id);
            return Err(format!("读取重放响应失败：{e}"));
        }
    };
    let res_size = bytes.len();
    let res_body = String::from_utf8(bytes.to_vec()).ok();

    traffic.complete(
        replay_id,
        parts.status.as_u16(),
        res_ct,
        res_headers,
        res_body,
        res_size,
        false,
    );

    Ok(replay_id)
}

fn sanitize_replay_headers(headers: &[(String, String)]) -> Vec<(String, String)> {
    headers
        .iter()
        .filter(|(name, _)| !is_hop_by_hop_replay_header(name))
        .cloned()
        .collect()
}

fn is_hop_by_hop_replay_header(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
            | "host"
            | "content-length"
    )
}

#[tauri::command]
fn traffic_clear(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.traffic.clear();
    Ok(())
}

#[tauri::command]
fn traffic_delete(state: tauri::State<'_, AppState>, ids: Vec<u64>) -> Result<(), String> {
    state.traffic.delete(&ids);
    Ok(())
}
