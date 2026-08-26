//! 证书管理命令：查看状态、生成/重新生成、复制 PEM、安装到系统信任区。

use serde::Serialize;

use crate::state::{self, AppState};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CertStatus {
    pub exists: bool,
    pub path: String,
    pub subject: String,
    pub not_after: String,
    pub cert_pem: String,
}

#[tauri::command]
pub fn cert_status(state: tauri::State<'_, AppState>) -> Result<CertStatus, String> {
    let cert_path = state.ca_path.join("cert.crt");
    let path = cert_path.to_string_lossy().into_owned();

    if !cert_path.exists() {
        return Ok(CertStatus {
            exists: false,
            path,
            subject: String::new(),
            not_after: String::new(),
            cert_pem: String::new(),
        });
    }

    let pem = std::fs::read_to_string(&cert_path).map_err(|e| format!("读取证书失败: {e}"))?;
    let (subject, not_after) = parse_cert_info(&pem);
    Ok(CertStatus {
        exists: true,
        path,
        subject,
        not_after,
        cert_pem: pem,
    })
}

/// 生成/重新生成 CA 证书。`force=true` 覆盖已存在证书并热替换内存中的 CA。
#[tauri::command]
pub fn cert_generate(state: tauri::State<'_, AppState>, force: bool) -> Result<(), String> {
    let cert_path = state.ca_path.join("cert.crt");
    if cert_path.exists() && !force {
        return Err("证书已存在，如需重新生成请勾选「强制重新生成」".to_string());
    }
    let new_ca = state::write_ca(&state.ca_path)?;
    *state.ca.lock().unwrap() = new_ca;
    Ok(())
}

#[tauri::command]
pub fn cert_get_pem(state: tauri::State<'_, AppState>) -> Result<String, String> {
    std::fs::read_to_string(state.ca_path.join("cert.crt"))
        .map_err(|e| format!("读取证书失败: {e}"))
}

/// 安装到系统信任区（Windows/Linux；失败时返回错误供前端提示）。
#[tauri::command]
pub fn cert_install_trust(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let pem = std::fs::read_to_string(state.ca_path.join("cert.crt"))
        .map_err(|e| format!("读取证书失败: {e}"))?;
    let mut pem_bytes: &[u8] = pem.as_bytes();
    let der = rustls_pemfile::certs(&mut pem_bytes).map_err(|e| format!("解析证书失败: {e}"))?;
    let der = der.first().ok_or_else(|| "证书文件为空".to_string())?;
    trust_cert::trust_cert(der)
}

/// 用 x509-parser 解析证书的 subject 与有效期。
fn parse_cert_info(pem: &str) -> (String, String) {
    use x509_parser::prelude::*;

    let mut pem_bytes: &[u8] = pem.as_bytes();
    let der = match rustls_pemfile::certs(&mut pem_bytes) {
        Ok(mut certs) if !certs.is_empty() => certs.remove(0),
        _ => return (String::new(), String::new()),
    };
    match X509Certificate::from_der(&der) {
        Ok((_, cert)) => {
            let subject = cert.subject().to_string();
            let not_after = cert.validity().not_after.to_datetime().to_string();
            (subject, not_after)
        }
        Err(_) => (String::new(), String::new()),
    }
}
