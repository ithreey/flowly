//! Windows 系统代理自动配置：启动代理时写入 WinINET 系统代理，停止时还原。
//!
//! MVP 仅实现 Windows（通过注册表 + InternetSetOption 刷新通知）；
//! 其它平台暂为空实现，后续可补 macOS(`networksetup`)/Linux。

/// 保存的旧系统代理配置，用于停止时还原。
pub struct SystemProxyGuard {
    old_enable: u32,
    old_server: String,
    old_override: Option<String>,
}

/// 设置系统代理指向 `addr`（如 `127.0.0.1:34567`），返回还原用的 guard。
#[cfg(target_os = "windows")]
pub fn set_system_proxy(addr: &str) -> Result<SystemProxyGuard, String> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};

    const SETTINGS_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Internet Settings";

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu
        .open_subkey_with_flags(SETTINGS_KEY, KEY_READ | KEY_WRITE)
        .map_err(|e| format!("打开代理注册表失败: {e}"))?;

    let old_enable: u32 = key.get_value("ProxyEnable").unwrap_or(0);
    let old_server: String = key.get_value("ProxyServer").unwrap_or_default();
    let old_override: Option<String> = key.get_value("ProxyOverride").ok();

    key.set_value("ProxyEnable", &1u32)
        .map_err(|e| format!("启用系统代理失败: {e}"))?;
    key.set_value("ProxyServer", &addr.to_string())
        .map_err(|e| format!("写入系统代理地址失败: {e}"))?;
    key.set_value(
        "ProxyOverride",
        &merge_local_proxy_override(old_override.as_deref()),
    )
    .map_err(|e| format!("写入本地代理绕过失败: {e}"))?;

    notify_proxy_change();
    Ok(SystemProxyGuard {
        old_enable,
        old_server,
        old_override,
    })
}

/// 如果系统代理残留为本应用监听地址，启动时先关闭它，避免 WebView 加载本地页面失败。
#[cfg(target_os = "windows")]
pub fn clear_stale_system_proxy(addr: &str) -> Result<(), String> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};

    const SETTINGS_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Internet Settings";

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu
        .open_subkey_with_flags(SETTINGS_KEY, KEY_READ | KEY_WRITE)
        .map_err(|e| format!("打开代理注册表失败: {e}"))?;

    let enable: u32 = key.get_value("ProxyEnable").unwrap_or(0);
    let server: String = key.get_value("ProxyServer").unwrap_or_default();
    if enable != 0 && proxy_server_uses_addr(&server, addr) {
        key.set_value("ProxyEnable", &0u32)
            .map_err(|e| format!("关闭遗留系统代理失败: {e}"))?;
        notify_proxy_change();
    }

    Ok(())
}

/// 还原系统代理到 guard 记录的状态。
#[cfg(target_os = "windows")]
pub fn restore_system_proxy(guard: &SystemProxyGuard) {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};

    const SETTINGS_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Internet Settings";

    if let Ok(hkcu) =
        RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(SETTINGS_KEY, KEY_READ | KEY_WRITE)
    {
        let _ = hkcu.set_value("ProxyEnable", &guard.old_enable);
        let _ = hkcu.set_value("ProxyServer", &guard.old_server);
        match &guard.old_override {
            Some(value) => {
                let _ = hkcu.set_value("ProxyOverride", value);
            }
            None => {
                let _ = hkcu.delete_value("ProxyOverride");
            }
        }
        notify_proxy_change();
    }
}

#[cfg(not(target_os = "windows"))]
pub fn set_system_proxy(_addr: &str) -> Result<SystemProxyGuard, String> {
    Err("当前平台暂不支持自动设置系统代理".to_string())
}

#[cfg(not(target_os = "windows"))]
pub fn restore_system_proxy(_guard: &SystemProxyGuard) {}

#[cfg(not(target_os = "windows"))]
pub fn clear_stale_system_proxy(_addr: &str) -> Result<(), String> {
    Ok(())
}

fn merge_local_proxy_override(existing: Option<&str>) -> String {
    let mut parts: Vec<String> = existing
        .unwrap_or_default()
        .split(';')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(ToString::to_string)
        .collect();

    for required in ["localhost", "127.0.0.1", "::1", "<local>"] {
        if !parts.iter().any(|part| part.eq_ignore_ascii_case(required)) {
            parts.push(required.to_string());
        }
    }

    parts.join(";")
}

fn proxy_server_uses_addr(proxy_server: &str, addr: &str) -> bool {
    proxy_server.split(';').any(|part| {
        let value = part
            .split_once('=')
            .map_or(part, |(_, value)| value)
            .trim()
            .trim_start_matches("http://")
            .trim_start_matches("https://");
        value.eq_ignore_ascii_case(addr)
    })
}

#[cfg(target_os = "windows")]
fn notify_proxy_change() {
    use windows_sys::Win32::Networking::WinInet::{
        INTERNET_OPTION_REFRESH, INTERNET_OPTION_SETTINGS_CHANGED, InternetSetOptionW,
    };

    unsafe {
        // 让 WinINET 立即感知代理配置变化，避免等待系统轮询。
        let _ = InternetSetOptionW(
            std::ptr::null_mut(),
            INTERNET_OPTION_SETTINGS_CHANGED,
            std::ptr::null_mut(),
            0,
        );
        let _ = InternetSetOptionW(
            std::ptr::null_mut(),
            INTERNET_OPTION_REFRESH,
            std::ptr::null_mut(),
            0,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{merge_local_proxy_override, proxy_server_uses_addr};

    #[test]
    fn merge_local_proxy_override_preserves_existing_and_adds_local_bypass() {
        let merged = merge_local_proxy_override(Some("example.com;<local>"));

        assert_eq!(merged, "example.com;<local>;localhost;127.0.0.1;::1");
    }

    #[test]
    fn merge_local_proxy_override_deduplicates_case_insensitively() {
        let merged = merge_local_proxy_override(Some("LOCALHOST;127.0.0.1"));

        assert_eq!(merged, "LOCALHOST;127.0.0.1;::1;<local>");
    }

    #[test]
    fn proxy_server_uses_addr_matches_direct_and_protocol_specific_settings() {
        assert!(proxy_server_uses_addr("127.0.0.1:34567", "127.0.0.1:34567"));
        assert!(proxy_server_uses_addr(
            "http=127.0.0.1:34567;https=127.0.0.1:34567",
            "127.0.0.1:34567"
        ));
        assert!(proxy_server_uses_addr(
            "http=http://127.0.0.1:34567",
            "127.0.0.1:34567"
        ));
        assert!(!proxy_server_uses_addr("127.0.0.1:7890", "127.0.0.1:34567"));
    }
}
