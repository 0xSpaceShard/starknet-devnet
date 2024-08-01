use axum::extract::State;
use axum::Json;

use super::extract_optional_json_from_request;
use crate::api::http::error::HttpApiError;
use crate::api::http::models::{DumpPath, DumpResponseBody, LoadPath};
use crate::api::http::{HttpApiHandler, HttpApiResult};
use crate::api::Api;

pub async fn dump(
    State(state): State<HttpApiHandler>,
    optional_path: Option<Json<DumpPath>>,
) -> HttpApiResult<Json<DumpResponseBody>> {
    dump_impl(&state.api, extract_optional_json_from_request(optional_path)).await.map(Json::from)
}

pub(crate) async fn dump_impl(
    api: &Api,
    path_wrapper: Option<DumpPath>,
) -> HttpApiResult<DumpResponseBody> {
    let starknet = api.starknet.lock().await;

    if starknet.config.dump_on.is_none() {
        return Err(HttpApiError::DumpError {
            msg: "Please provide --dump-on mode on startup.".to_string(),
        });
    }

    let path = path_wrapper
        .as_ref()
        .map(|DumpPath { path }| path.as_str())
        .or_else(|| starknet.config.dump_path.as_deref())
        .unwrap_or("");

    if path.is_empty() {
        let json_dump = starknet.read_dump_events();
        Ok(Some(json_dump.clone()))
    } else {
        starknet
            .dump_events(path)
            .map_err(|err| HttpApiError::DumpError { msg: err.to_string() })?;
        Ok(None)
    }
}

pub async fn load(
    State(state): State<HttpApiHandler>,
    Json(path_wrapper): Json<LoadPath>,
) -> HttpApiResult<()> {
    load_impl(&state.api, path_wrapper).await
}

pub(crate) async fn load_impl(api: &Api, path_wrapper: LoadPath) -> HttpApiResult<()> {
    let mut starknet = api.starknet.lock().await;

    // necessary to restart before loading
    starknet.restart().map_err(|e| HttpApiError::RestartError { msg: e.to_string() })?;

    match starknet.load_events(&path_wrapper.path) {
        Ok(events) => {
            starknet.re_execute(events).map_err(|e| HttpApiError::ReExecutionError(e.to_string()))
        }
        Err(starknet_core::error::Error::FileNotFound) => Err(HttpApiError::FileNotFound),
        Err(e) => Err(HttpApiError::LoadError(e.to_string())),
    }
}
