use thiserror::Error;

#[derive(Error, Debug)]
pub enum TestError {
    #[error("No free ports")]
    NoFreePorts,

    #[error("Could not parse URL")]
    UrlParseError(#[from] url::ParseError),

    #[error("Invalid URI")]
    InvalidUri(#[from] hyper::http::uri::InvalidUri),

    #[error("Could not start Devnet. Make sure you've built it with: `cargo build --release`")]
    DevnetNotStartable,

    #[error("Could not start Anvil")]
    AnvilNotStartable,

    #[error("Ethers error: {0}")]
    EthersError(String),
}
