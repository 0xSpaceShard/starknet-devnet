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
    path: Option<DumpPath>,
) -> HttpApiResult<DumpResponseBody> {
    let starknet = api.starknet.lock().await;

    if starknet.config.dump_on.is_none() {
        return Err(HttpApiError::DumpError {
            msg: "Please provide --dump-on mode on startup.".to_string(),
        });
    }

    let path = path.map_or(String::new(), |s| s.path.clone());

    if path.is_empty() {
        match &starknet.config.dump_path {
            Some(path) => {
                // dump_path is present
                starknet
                    .dump_events_custom_path(Some(path.clone()))
                    .map_err(|err| HttpApiError::DumpError { msg: err.to_string() })?;
                Ok(None)
            }
            None => {
                // dump_path is not present
                let json_dump = starknet.read_dump_events();
                Ok(Some(json_dump.clone()))
            }
        }
    } else {
        starknet
            .dump_events_custom_path(Some(path))
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
