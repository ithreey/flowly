use std::time::Instant;
use tokio::time::{timeout, Duration};

use mitm_core::{
    http_client::{gen_client, HttpClient},
    hyper::{body::to_bytes, Body, Request, Uri},
};
use serde::Serialize;

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
    method: String,
    url: String,
    headers: Vec<(String, String)>,
    body: Option<Vec<u8>>,
) -> Result<SendResponse, String> {
    let uri: Uri = url.parse().map_err(|e| format!("URL 无效: {e}"))?;

    let mut builder = Request::builder().method(method.as_str()).uri(&uri);
    for (name, value) in &headers {
        builder = builder.header(name.as_str(), value.as_str());
    }
    let request = builder
        .body(Body::from(body.unwrap_or_default()))
        .map_err(|e| format!("构造请求失败: {e}"))?;

    // 始终使用直连客户端（信任所有证书）。
    // 代理路由需要 CONNECT 隧道支持，后续完善。
    let client = gen_client(None).map_err(|e| format!("创建客户端失败: {e}"))?;

    let start = Instant::now();
    let response = match client {
        HttpClient::Https(c) => timeout(Duration::from_secs(30), c.request(request))
            .await
            .map_err(|_| "请求超时（30s）".to_string())?
            .map_err(|e| format!("请求失败: {e}"))?,
        HttpClient::Proxy(_) => return Err("代理模式暂不支持".to_string()),
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
    let bytes = timeout(Duration::from_secs(30), to_bytes(body))
        .await
        .map_err(|_| "读取响应超时（30s）".to_string())?
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
