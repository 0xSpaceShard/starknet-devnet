use axum::extract::State;
use axum::Json;
use serde::Serialize;
use starknet_core::starknet::starknet_config::StarknetConfig;

use super::error::HttpApiError;
use super::{HttpApiHandler, HttpApiResult};
use crate::ServerConfig;

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
pub async fn restart(State(state): State<HttpApiHandler>) -> HttpApiResult<()> {
    state
        .api
        .starknet
        .write()
        .await
        .restart()
        .map_err(|err| HttpApiError::RestartError { msg: err.to_string() })?;
    Ok(())
}

#[derive(Serialize)]
pub struct DevnetConfig {
    #[serde(flatten)]
    starknet_config: StarknetConfig,
    server_config: ServerConfig,
}

/// Devnet config
pub async fn get_devnet_config(
    State(state): State<HttpApiHandler>,
) -> HttpApiResult<Json<DevnetConfig>> {
    Ok(Json(DevnetConfig {
        starknet_config: state.api.starknet.read().await.config.clone(),
        server_config: state.server_config.clone(),
    }))
}
