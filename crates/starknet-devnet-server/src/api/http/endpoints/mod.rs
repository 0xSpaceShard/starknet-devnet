use axum::{Extension, Json};

use super::error::HttpApiError;
use super::models::ForkStatus;
use super::{HttpApiHandler, HttpApiResult};

/// Dumping and loading
pub mod dump_load;

/// Postman
pub mod postman;

/// Blocks
pub mod blocks;

/// Time
pub mod time;

/// Accounts
pub mod accounts;

/// Mint token - Local faucet
pub mod mint_token;

/// Is alive
pub async fn is_alive() -> HttpApiResult<String> {
    Ok("Alive!!!".to_string())
}

/// Restart
pub async fn restart(Extension(state): Extension<HttpApiHandler>) -> HttpApiResult<()> {
    state
        .api
        .starknet
        .write()
        .await
        .restart()
        .map_err(|err| HttpApiError::RestartError { msg: err.to_string() })?;
    Ok(())
}

/// Fork
pub async fn get_fork_status(
    Extension(state): Extension<HttpApiHandler>,
) -> HttpApiResult<Json<ForkStatus>> {
    let fork_config = &state.api.starknet.read().await.config.fork_config;
    Ok(Json(ForkStatus {
        url: fork_config.url.as_ref().map(|url| url.to_string()),
        block: fork_config.block,
    }))
}
