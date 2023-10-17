use axum::{Extension, Json};

use crate::api::http::error::HttpApiError;
use crate::api::http::models::{DumpPath, LoadPath};
use crate::api::http::{HttpApiHandler, HttpApiResult};

pub(crate) async fn dump(
    Json(path): Json<DumpPath>,
    Extension(state): Extension<HttpApiHandler>,
) -> HttpApiResult<()> {
    let starknet = state.api.starknet.write().await;
    match path.path {
        None => {
            // path not present
            starknet
                .dump_transactions()
                .map_err(|err| HttpApiError::DumpError { msg: err.to_string() })?;
            Ok(())
        }
        Some(path) => {
            if !path.is_empty() {
                // path is present and it's not empty
                starknet
                    .dump_transactions_custom_path(Some(path))
                    .map_err(|err| HttpApiError::DumpError { msg: err.to_string() })?;
                Ok(())
            } else {
                // path is present but it's empty
                starknet
                    .dump_transactions()
                    .map_err(|err| HttpApiError::DumpError { msg: err.to_string() })?;
                Ok(())
            }
        }
    }
}

pub(crate) async fn load(
    Json(path): Json<LoadPath>,
    Extension(state): Extension<HttpApiHandler>,
) -> HttpApiResult<()> {
    let file_path = std::path::Path::new(&path.path);
    if path.path.is_empty() || !file_path.exists() {
        return Err(HttpApiError::FileNotFound);
    }

    let mut starknet = state.api.starknet.write().await;
    let transactions = starknet
        .load_transactions_custom_path(Some(path.path))
        .map_err(|_| HttpApiError::LoadError)?;
    starknet.re_execute(transactions).map_err(|_| HttpApiError::ReExecutionError)?;

    Ok(())
}
