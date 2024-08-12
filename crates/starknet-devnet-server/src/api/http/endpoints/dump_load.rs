use axum::extract::State;
use axum::Json;

use super::extract_optional_json_from_request;
use crate::api::http::error::HttpApiError;
use crate::api::http::models::{DumpPath, DumpResponseBody};
use crate::api::http::{HttpApiHandler, HttpApiResult};
use crate::api::Api;
use crate::dump::dump_events;

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
        .map(|DumpPath { path }| path.clone())
        .or_else(|| starknet.config.dump_path.clone())
        .unwrap_or_default();

    drop(starknet);
    let dumpable_events = api.dumpable_events.lock().await;

    if path.is_empty() {
        Ok(Some(dumpable_events.clone()))
    } else {
        dump_events(&dumpable_events, &path)
            .map_err(|err| HttpApiError::DumpError { msg: err.to_string() })?;
        Ok(None)
    }
}
