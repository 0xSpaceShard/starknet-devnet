use thiserror::Error;
#[derive(Error, Debug)]
pub enum TestError {
    #[error("Could not parse URL")]
    UrlParseError(#[from] url::ParseError),

    #[error("Invalid URI")]
    InvalidUri(#[from] axum::http::uri::InvalidUri),

    #[error("Could not start Devnet: {0}")]
    DevnetNotStartable(String),

    #[error("Too many ports occupied: {0:?}")]
    TooManyPorts(Vec<u16>),

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
