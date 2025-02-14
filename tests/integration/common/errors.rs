use core::fmt;
use std::borrow::Cow;

use serde::{self, Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TestError {
    #[error("Could not parse URL")]
    UrlParseError(#[from] url::ParseError),

    #[error("Invalid URI")]
    InvalidUri(#[from] axum::http::uri::InvalidUri),

    #[error("Could not start Devnet. Make sure you built it with `cargo build --release`: {0}")]
    DevnetNotStartable(String),

    #[error("Could not start Anvil")]
    AnvilNotStartable,

    #[error("Ethers error: {0}")]
    EthersError(String),
}

#[derive(Error, Debug)]
pub enum ReqwestError {
    #[error(transparent)]
    Error(#[from] reqwest::Error),
    #[error("Error with message: {message}")]
    ErrorWithMessage { error: reqwest::Error, message: String },
}

impl ReqwestError {
    pub fn reqwest_error(&self) -> &reqwest::Error {
        match self {
            ReqwestError::Error(e) => e,
            ReqwestError::ErrorWithMessage { error, .. } => error,
        }
    }

    pub fn status(&self) -> reqwest::StatusCode {
        self.reqwest_error().status().unwrap()
    }

    pub fn error_message(&self) -> String {
        match self {
            ReqwestError::Error(_) => "".to_string(),
            ReqwestError::ErrorWithMessage { message, .. } => message.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RpcError {
    pub code: i64,
    /// error message
    pub message: Cow<'static, str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl fmt::Display for RpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
