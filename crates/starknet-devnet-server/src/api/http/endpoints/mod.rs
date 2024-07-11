use axum::extract::State;
use axum::Json;
use serde::Serialize;
use starknet_core::starknet::starknet_config::StarknetConfig;

use super::error::HttpApiError;
use super::{HttpApiHandler, HttpApiResult};
use crate::api::Api;
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
    restart_impl(&state.api).await
}

pub(crate) async fn restart_impl(api: &Api) -> HttpApiResult<()> {
    api.starknet
        .write()
        .await
        .restart()
        .map_err(|err| HttpApiError::RestartError { msg: err.to_string() })?;

    Ok(())
}

#[derive(Serialize)]
pub struct DevnetConfig {
    #[serde(flatten)]
    pub(crate) starknet_config: StarknetConfig,
    pub(crate) server_config: ServerConfig,
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

pub(crate) fn extract_optional_json_from_request<T>(optional_json: Option<Json<T>>) -> Option<T> {
    optional_json.map(|json| Some(json.0)).unwrap_or(Option::None)
}
