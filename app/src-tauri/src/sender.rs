use std::time::Instant;

use mitm_core::{
    http_client::{gen_client, HttpClient},
    hyper::{body::to_bytes, Body, Request, Uri},
};
use serde::Serialize;

use crate::state::AppState;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SendResponse {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub duration_ms: u64,
}

#[tauri::command]
pub async fn send_request(
    state: tauri::State<'_, AppState>,
    method: String,
    url: String,
    headers: Vec<(String, String)>,
    body: Option<Vec<u8>>,
    through_proxy: bool,
) -> Result<SendResponse, String> {
    let uri: Uri = url.parse().map_err(|e| format!("URL 无效: {e}"))?;

    let mut builder = Request::builder().method(method.as_str()).uri(&uri);
    for (name, value) in &headers {
        builder = builder.header(name.as_str(), value.as_str());
    }
    let request = builder
        .body(Body::from(body.unwrap_or_default()))
        .map_err(|e| format!("构造请求失败: {e}"))?;

    let client = if through_proxy {
        let proxy_addr = {
            let guard = state.proxy.lock().unwrap();
            guard
                .as_ref()
                .map(|h| h.listen_addr.clone())
                .ok_or_else(|| "代理未启动，无法经过代理发送。请先启动代理或取消勾选。".to_string())?
        };
        let proxy_uri: Uri = format!("http://{proxy_addr}")
            .parse()
            .map_err(|e| format!("代理地址无效: {e}"))?;
        let proxy =
            flowly::hyper_proxy::Proxy::new(flowly::hyper_proxy::Intercept::All, proxy_uri);
        gen_client(Some(proxy)).map_err(|e| format!("创建代理客户端失败: {e}"))?
    } else {
        gen_client(None).map_err(|e| format!("创建客户端失败: {e}"))?
    };

    let start = Instant::now();
    let response = match client {
        HttpClient::Https(c) => c
            .request(request)
            .await
            .map_err(|e| format!("请求失败: {e}"))?,
        HttpClient::Proxy(c) => c
            .request(request)
            .await
            .map_err(|e| format!("请求失败: {e}"))?,
    };
    let duration_ms = start.elapsed().as_millis() as u64;

    let (parts, body) = response.into_parts();
    let resp_headers: Vec<(String, String)> = parts
        .headers
        .iter()
        .map(|(k, v)| {
            (
                k.as_str().to_string(),
                v.to_str().unwrap_or_default().to_string(),
            )
        })
        .collect();
    let bytes = to_bytes(body)
        .await
        .map_err(|e| format!("读取响应失败: {e}"))?;

    let status_text = parts
        .status
        .canonical_reason()
        .unwrap_or("Unknown")
        .to_string();

    Ok(SendResponse {
        status: parts.status.as_u16(),
        status_text,
        headers: resp_headers,
        body: bytes.to_vec(),
        duration_ms,
    })
}
