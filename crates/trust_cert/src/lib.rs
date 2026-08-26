#[cfg(windows)]
mod windows;

#[cfg(target_os = "linux")]
mod linux;

/// 把 CA 证书（DER 编码）安装到系统信任区。返回错误时由调用方（GUI/CLI）提示用户。
pub fn trust_cert(der: &[u8]) -> Result<(), String> {
    #[cfg(windows)]
    {
        return windows::install_cert(der);
    }
    #[cfg(target_os = "linux")]
    {
        return linux::install_cert(der);
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        let _ = der;
        Err("当前平台暂不支持自动安装证书".to_string())
    }
}
