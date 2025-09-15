use crate::api::Api;
use crate::api::error::ApiError;
use crate::api::models::{DumpPath, DumpResponseBody};
use crate::dump_util::dump_events;

pub(crate) async fn dump_impl(
    api: &Api,
    path_wrapper: Option<DumpPath>,
) -> Result<DumpResponseBody, ApiError> {
    let starknet = api.starknet.lock().await;

    if starknet.config.dump_on.is_none() {
        return Err(ApiError::DumpError {
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
            .map_err(|err| ApiError::DumpError { msg: err.to_string() })?;
        Ok(None)
    }
}
