use std::{
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

use mitm_core::{CertificateAuthority, handler::MitmFilter};
use rule::{Rule, RuleHandlerCtx};
use tauri::Manager;
use x509_parser::prelude::{FromDer, X509Certificate};

use crate::config::AppConfig;
use crate::gui_handler::GuiHandler;
use crate::proxy_ctrl::ProxyHandle;
use crate::traffic::SharedTraffic;

/// Tauri 全局状态。代理启停、规则热重载、证书、流量记录都经由这里共享。
pub struct AppState {
    pub proxy: std::sync::Mutex<Option<ProxyHandle>>,
    /// 用 Mutex 以便 `cert_generate` 重新生成后热替换。
    pub ca: std::sync::Mutex<CertificateAuthority>,
    /// 与 `handler` 共享同一锁，预留给阶段 3 的规则热重载命令使用。
    #[allow(dead_code)]
    pub rules: Arc<RwLock<Vec<Rule>>>,
    pub mitm_filter: Arc<MitmFilter<RuleHandlerCtx>>,
    /// 流量捕获 handler（内部包装 `RuleHttpHandler`）。
    pub handler: GuiHandler,
    pub config: Arc<RwLock<AppConfig>>,
    pub traffic: SharedTraffic,
    /// 规则文件路径（规则保存/热应用时写回）。
    pub rules_path: PathBuf,
    /// CA 证书目录（cert.crt / private.key）。
    pub ca_path: PathBuf,
    /// 配置持久化文件（config.json）。
    pub config_path: PathBuf,
    /// 历史记录持久化文件（history.json）。
    pub history_path: PathBuf,
    /// 发送器历史记录。
    pub history: crate::history::HistoryStore,
}

/// 加载或生成 CA 证书，写入 `app_data_dir/ca/`。
///
/// 与 `src/ca.rs::gen_ca` 的差异：目录参数化 + `create_dir_all` 幂等，
/// 避免重复运行时 panic。
pub fn ensure_ca(app_data_dir: &Path) -> Result<CertificateAuthority, String> {
    let ca_dir = app_data_dir.join("ca");
    let cert_path = ca_dir.join("cert.crt");
    let key_path = ca_dir.join("private.key");

    if !cert_path.exists() || !key_path.exists() || !ca_cert_matches_brand(&cert_path) {
        write_ca(&ca_dir)?;
    }

    load_ca(&cert_path, &key_path)
}

/// 生成新的 CA 证书并写入 `ca_dir`（幂等 `create_dir_all`），返回重建的 CA。
pub fn write_ca(ca_dir: &Path) -> Result<CertificateAuthority, String> {
    std::fs::create_dir_all(ca_dir).map_err(|e| format!("创建 CA 目录失败: {e}"))?;
    let cert = CertificateAuthority::gen_ca().map_err(|e| format!("生成 CA 失败: {e}"))?;
    let cert_pem = cert
        .serialize_pem()
        .map_err(|e| format!("序列化证书失败: {e}"))?;
    let key_pem = cert.serialize_private_key_pem();
    std::fs::write(ca_dir.join("cert.crt"), &cert_pem).map_err(|e| format!("写证书失败: {e}"))?;
    std::fs::write(ca_dir.join("private.key"), &key_pem).map_err(|e| format!("写私钥失败: {e}"))?;
    load_ca(&ca_dir.join("cert.crt"), &ca_dir.join("private.key"))
}

fn ca_cert_matches_brand(cert_path: &Path) -> bool {
    let Ok(ca_cert_bytes) = std::fs::read(cert_path) else {
        return false;
    };
    let Ok(certs) = rustls_pemfile::certs(&mut ca_cert_bytes.as_slice()) else {
        return false;
    };
    let Some(cert) = certs.first() else {
        return false;
    };
    let Ok((_, parsed)) = X509Certificate::from_der(cert) else {
        return false;
    };

    let matches_brand = parsed
        .subject()
        .iter_common_name()
        .any(|cn| matches!(cn.as_str(), Ok(value) if value == "Flowly"));
    matches_brand
}

fn load_ca(cert_path: &Path, key_path: &Path) -> Result<CertificateAuthority, String> {
    let private_key_bytes = std::fs::read(key_path).map_err(|e| format!("读私钥失败: {e}"))?;
    let private_key = rustls_pemfile::pkcs8_private_keys(&mut private_key_bytes.as_slice())
        .map_err(|e| format!("解析私钥失败: {e}"))?;
    if private_key.is_empty() {
        return Err("私钥文件为空".to_string());
    }
    let private_key = rustls::PrivateKey(private_key[0].clone());

    let ca_cert_bytes = std::fs::read(cert_path).map_err(|e| format!("读证书失败: {e}"))?;
    let ca_cert = rustls_pemfile::certs(&mut ca_cert_bytes.as_slice())
        .map_err(|e| format!("解析证书失败: {e}"))?;
    if ca_cert.is_empty() {
        return Err("证书文件为空".to_string());
    }
    let ca_cert = rustls::Certificate(ca_cert[0].clone());

    CertificateAuthority::new(
        private_key,
        ca_cert,
        String::from_utf8(ca_cert_bytes).map_err(|e| format!("证书内容非 UTF-8: {e}"))?,
        1_000,
    )
    .map_err(|e| format!("创建 CertificateAuthority 失败: {e}"))
}

/// 加载默认规则。规则文件不存在时创建空规则集。
///
/// 返回可热重载的规则存储与 MITM 主机过滤器模式列表。
pub fn ensure_rules(app_data_dir: &Path) -> Result<(Arc<RwLock<Vec<Rule>>>, Vec<String>), String> {
    let rules_path = app_data_dir.join("rules.json");
    if !rules_path.exists() {
        std::fs::write(&rules_path, "[]\n").map_err(|e| format!("创建规则文件失败: {e}"))?;
    }

    let rules_path_str = rules_path.to_string_lossy().to_string();
    let (rules, mitm_filters) = flowly::file::load_rules_amd_mitm_filters(&rules_path_str)
        .map_err(|e| format!("加载规则失败: {e}"))?;
    Ok((Arc::new(RwLock::new(rules)), mitm_filters))
}

/// 数据目录：`app_data_dir/ca` 存证书，`app_data_dir/rules.yaml` 存规则。
pub fn app_data_dir(app: &tauri::App) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("定位应用数据目录失败: {e}"))?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建应用数据目录失败: {e}"))?;
    Ok(dir)
}
