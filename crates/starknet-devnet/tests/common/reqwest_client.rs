use std::any::TypeId;

use serde::de::DeserializeOwned;
use serde::Serialize;

use super::errors::ReqwestError;

#[derive(Clone, Debug)]
pub struct HttpEmptyResponseBody;
#[derive(Clone)]
pub struct ReqwestClient {
    url: String,
}

impl ReqwestClient {
    pub fn new(url: String) -> Self {
        Self { url }
    }

    async fn get_response<TParam>(
        &self,
        path: &str,
        body: TParam,
    ) -> Result<reqwest::Response, ReqwestError>
    where
        TParam: Serialize + Send + Sync + 'static,
    {
        let url = format!("{}{}", self.url, path);
        let request_builder = reqwest::Client::new().post(&url);
        if TypeId::of::<TParam>() == TypeId::of::<()>() {
            request_builder.send().await.map_err(|err| ReqwestError::Error(err))
        } else {
            request_builder.json(&body).send().await.map_err(|err| ReqwestError::Error(err))
        }
    }
}

#[async_trait::async_trait]
pub trait ReqwestSender<TParam, TResponse>: Clone + Send + Sync + 'static {
    /// Sends a POST request to the devnet with the given path and body (`TParam`), and returns the
    /// response
    ///
    /// # Arguments
    /// * `path` - The path to send the request to
    /// * `body` - The body of the request
    ///
    /// # Returns
    /// The response from the devnet in the form of the type `TResponse`
    /// If http status is not success it will return a `super::errors::ReqwestError`
    async fn post_json_async(&self, path: &str, body: TParam) -> Result<TResponse, ReqwestError>;
}

#[async_trait::async_trait]
impl<TParam, TResponse> ReqwestSender<TParam, TResponse> for ReqwestClient
where
    TParam: Serialize + Send + Sync + 'static,
    TResponse: DeserializeOwned + 'static,
{
    async fn post_json_async(&self, path: &str, body: TParam) -> Result<TResponse, ReqwestError> {
        let response = self.get_response(path, body).await?;

        if response.status().is_success() {
            return response.json::<TResponse>().await.map_err(|err| err.into());
        } else {
            if response.content_length().unwrap_or(0) > 0 {
                let error = response.error_for_status_ref().unwrap_err();
                let error_message = response.text().await.unwrap();
                return Err(ReqwestError::ErrorWithMessage { error, message: error_message });
            }
            return Err(response.error_for_status().unwrap_err().into());
        }
    }
}

#[async_trait::async_trait]
impl<TParam> ReqwestSender<TParam, HttpEmptyResponseBody> for ReqwestClient
where
    TParam: Serialize + Send + Sync + 'static,
{
    async fn post_json_async(
        &self,
        path: &str,
        body: TParam,
    ) -> Result<HttpEmptyResponseBody, ReqwestError> {
        let response = self.get_response(path, body).await?;

        if response.status().is_success() {
            return Ok(HttpEmptyResponseBody {});
        } else {
            if response.content_length().unwrap_or(0) > 0 {
                let error = response.error_for_status_ref().unwrap_err();
                let error_message = response.text().await.unwrap();
                return Err(ReqwestError::ErrorWithMessage { error, message: error_message });
            }
            return Err(response.error_for_status().unwrap_err().into());
        }
    }
}
