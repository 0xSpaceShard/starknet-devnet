use axum::extract::State;
use axum::Json;

use crate::api::http::error::HttpApiError;
use crate::api::http::models::{DumpPath, LoadPath};
use crate::api::http::{HttpApiHandler, HttpApiResult};
use crate::api::Api;

pub async fn dump(
    State(state): State<HttpApiHandler>,
    Json(path): Json<DumpPath>,
) -> HttpApiResult<()> {
    dump_impl(&state.api, path).await
}

pub(crate) async fn dump_impl(api: &Api, path: DumpPath) -> HttpApiResult<()> {
    let starknet = api.starknet.write().await;

    if starknet.config.dump_on.is_none() {
        return Err(HttpApiError::DumpError {
            msg: "Please provide --dump-on mode on startup.".to_string(),
        });
    }

    match path.path {
        None => {
            // path not present
            starknet
                .dump_events()
                .map_err(|err| HttpApiError::DumpError { msg: err.to_string() })?;
            Ok(())
        }
        Some(path) => {
            if !path.is_empty() {
                // path is present and it's not empty
                starknet
                    .dump_events_custom_path(Some(path))
                    .map_err(|err| HttpApiError::DumpError { msg: err.to_string() })?;
                Ok(())
            } else {
                // path is present but it's empty
                starknet
                    .dump_events()
                    .map_err(|err| HttpApiError::DumpError { msg: err.to_string() })?;
                Ok(())
            }
        }
    }
}

pub async fn load(
    State(state): State<HttpApiHandler>,
    Json(path): Json<LoadPath>,
) -> HttpApiResult<()> {
    load_impl(&state.api, path).await
}

pub(crate) async fn load_impl(api: &Api, path: LoadPath) -> HttpApiResult<()> {
    let file_path = std::path::Path::new(&path.path);
    if path.path.is_empty() || !file_path.exists() {
        return Err(HttpApiError::FileNotFound);
    }

    let mut starknet = api.starknet.write().await;
    let events =
        starknet.load_events_custom_path(Some(path.path)).map_err(|_| HttpApiError::LoadError)?;
    starknet.re_execute(events).map_err(|_| HttpApiError::ReExecutionError)?;

    Ok(())
}
