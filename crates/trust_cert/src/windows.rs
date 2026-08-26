use windows::{
    Win32::Security::Cryptography::{
        CERT_QUERY_ENCODING_TYPE, CERT_STORE_ADD_REPLACE_EXISTING,
        CertAddEncodedCertificateToStore, CertCloseStore, CertOpenSystemStoreW, HCRYPTPROV_LEGACY,
        PKCS_7_ASN_ENCODING, X509_ASN_ENCODING,
    },
    w,
};

pub fn install_cert(der: &[u8]) -> Result<(), String> {
    unsafe {
        // get root store
        let store = CertOpenSystemStoreW(HCRYPTPROV_LEGACY(0), w!("ROOT"))
            .map_err(|e| format!("打开系统根证书存储失败: {e}"))?;

        let encoding = CERT_QUERY_ENCODING_TYPE(X509_ASN_ENCODING.0 | PKCS_7_ASN_ENCODING.0);

        // add cert
        if !CertAddEncodedCertificateToStore(
            store,
            encoding,
            der,
            CERT_STORE_ADD_REPLACE_EXISTING,
            None,
        )
        .as_bool()
        {
            let _ = CertCloseStore(store, 0);
            return Err("将证书写入系统根证书存储失败".to_string());
        }

        if !CertCloseStore(store, 0).as_bool() {
            return Err("关闭证书存储失败".to_string());
        }
    }
    Ok(())
}
