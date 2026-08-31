use serde::{Deserialize, Serialize};
use std::path::Path;

/// 应用运行配置。持久化到 `config.json`。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub listen_addr: String,
    pub upstream_proxy: Option<String>,
    pub capture_body: bool,
    pub max_body_size: usize,
    /// 启动代理时自动设置本机系统代理，停止时还原。
    pub auto_system_proxy: bool,
    /// 全局 HTTPS MITM 域名模式，一行一个，如 `*`、`*.example.com`。
    #[serde(default)]
    pub mitm_hosts: Vec<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            listen_addr: "0.0.0.0:34567".to_string(),
            upstream_proxy: None,
            capture_body: true,
            max_body_size: 256 * 1024,
            auto_system_proxy: true,
            mitm_hosts: vec![],
        }
    }
}

/// 从 `config.json` 加载配置，文件不存在或解析失败时返回默认值。
pub fn load_config(path: &Path) -> AppConfig {
    if let Ok(content) = std::fs::read_to_string(path) {
        if let Ok(cfg) = serde_json::from_str(&content) {
            return cfg;
        }
    }
    AppConfig::default()
}

pub fn has_mitm_hosts_config(path: &Path) -> bool {
    let Ok(content) = std::fs::read_to_string(path) else {
        return false;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) else {
        return false;
    };
    value.get("mitmHosts").is_some()
}

/// 持久化配置到 `config.json`。
pub fn save_config(path: &Path, cfg: &AppConfig) -> Result<(), String> {
    let json = serde_json::to_string_pretty(cfg).map_err(|e| format!("序列化配置失败: {e}"))?;
    std::fs::write(path, json).map_err(|e| format!("写配置文件失败: {e}"))
}

#[cfg(test)]
mod tests {
    use super::AppConfig;

    #[test]
    fn default_config_listens_on_all_ipv4_interfaces() {
        assert_eq!(AppConfig::default().listen_addr, "0.0.0.0:34567");
    }
}
