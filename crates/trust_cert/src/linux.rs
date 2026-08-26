use base64::{Engine, engine::general_purpose::STANDARD};
use nix::unistd::getegid;
use std::{env, fs, path::Path, process::Command};

pub fn install_cert(der: &[u8]) -> Result<(), String> {
    if getegid().as_raw() != 0 {
        return Err("需要 root 权限".to_string());
    }

    let (system_trust_filename, trust_cmd, trust_cmd_args) = {
        if path_exist("/etc/pki/ca-trust/source/anchors/") {
            (
                "/etc/pki/ca-trust/source/anchors/{cert-name}.pem",
                "update-ca-trust",
                vec!["extract"],
            )
        } else if path_exist("/usr/local/share/ca-certificates/") {
            (
                "/usr/local/share/ca-certificates/{cert-name}.crt",
                "update-ca-certificates",
                vec![],
            )
        } else if path_exist("/etc/ca-certificates/trust-source/anchors/") {
            (
                "/etc/ca-certificates/trust-source/anchors/{cert-name}.crt",
                "trust",
                vec!["extract-compat"],
            )
        } else if path_exist("/usr/share/pki/trust/anchors") {
            (
                "/usr/share/pki/trust/anchors/{cert-name}.pem",
                "update-ca-certificates",
                vec![],
            )
        } else {
            ("Flowly.pem", "", vec![])
        }
    };

    let cert = der_to_pem(der);
    let system_trust_name = system_trust_filename.replace("{cert-name}", "Flowly");
    fs::write(&system_trust_name, &cert)
        .map_err(|e| format!("写入证书到 {} 失败: {e}", system_trust_name))?;

    if trust_cmd.is_empty() {
        let cert_path = Path::new(&get_ca_root()).join("Flowly.pem");
        Err(format!(
            "此 Linux 发行版暂不支持自动安装，可手动安装到 {}",
            cert_path.to_str().unwrap_or("")
        ))
    } else {
        Command::new(trust_cmd)
            .args(trust_cmd_args)
            .status()
            .map_err(|e| format!("执行 {} 失败: {e}", trust_cmd))?;
        Ok(())
    }
}

fn get_ca_root() -> String {
    if let Ok(v) = env::var("CAROOT") {
        return v;
    }

    let mut dir = {
        if let Ok(v) = env::var("XDG_DATA_HOME") {
            return v;
        }
        if let Ok(v) = env::var("HOME") {
            return Path::new(&v)
                .join(".local")
                .join("share")
                .to_str()
                .map(|s| s.to_string())
                .unwrap();
        }
        String::new()
    };

    if !dir.is_empty() {
        dir = Path::new(&dir)
            .join("mitm")
            .into_os_string()
            .into_string()
            .unwrap()
    }

    dir
}

#[inline]
fn path_exist(path: &str) -> bool {
    Path::new(path).exists()
}

/// DER → PEM（每 64 字符换行）。
fn der_to_pem(der: &[u8]) -> String {
    let b64 = STANDARD.encode(der);
    let mut pem = String::from("-----BEGIN CERTIFICATE-----\n");
    for chunk in b64.as_bytes().chunks(64) {
        pem.push_str(std::str::from_utf8(chunk).unwrap());
        pem.push('\n');
    }
    pem.push_str("-----END CERTIFICATE-----\n");
    pem
}
