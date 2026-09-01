use rcgen::RcgenError;
use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid CA")]
    Tls(#[from] RcgenError),
    #[error("network error")]
    Hyper(#[from] hyper::Error),
    #[cfg(feature = "request-native-tls")]
    #[error("TlsConnector error")]
    TlsConnector(#[from] hyper_tls::native_tls::Error),
    #[error("IO error")]
    IO(#[from] io::Error),
    #[error("unable to decode response body")]
    Decode,
    #[error("unknown error")]
    Unknown,
}
