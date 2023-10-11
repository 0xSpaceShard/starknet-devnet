use axum::{Extension, Json};

use crate::api::http::error::HttpApiError;
use crate::api::http::models::{DumpResponse, LoadResponse, Path};
use crate::api::http::{HttpApiHandler, HttpApiResult};

pub(crate) async fn dump(
    Json(path): Json<Path>,
    Extension(state): Extension<HttpApiHandler>,
) -> HttpApiResult<Json<DumpResponse>> {
    if path.path.is_empty() {
        return Err(HttpApiError::FileNotFound);
    }

    let starknet = state.api.starknet.write().await;
    starknet
        .dump_transactions_custom_path(Some(path.path.clone()))
        .map_err(|_| HttpApiError::DumpError)?;

    Ok(Json(DumpResponse { path: path.path }))
}

pub(crate) async fn load(
    Json(path): Json<Path>,
    Extension(state): Extension<HttpApiHandler>,
) -> HttpApiResult<Json<LoadResponse>> {
    let file_path = std::path::Path::new(&path.path);
    if path.path.is_empty() || !file_path.exists() {
        return Err(HttpApiError::FileNotFound);
    }

    let mut starknet = state.api.starknet.write().await;
    let transactions = starknet
        .load_transactions_custom_path(Some(path.path.clone()))
        .map_err(|_| HttpApiError::LoadError)?;
    starknet.re_execute(transactions).map_err(|_| HttpApiError::ReExecutionError)?;

    Ok(Json(LoadResponse { path: path.path }))
}
