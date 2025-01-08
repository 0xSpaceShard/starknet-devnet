use std::any::TypeId;

use reqwest::Method;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::json;

use super::errors::ReqwestError;

#[derive(Clone, Debug)]
pub struct HttpEmptyResponseBody;
#[derive(Clone, Debug)]
pub struct ReqwestClient {
    url: String,
    reqwest_client: reqwest::Client,
}

impl ReqwestClient {
    pub fn new(url: String, reqwest_client: reqwest::Client) -> Self {
        Self { url, reqwest_client }
    }

    async fn get_response<TParam>(
        &self,
        path: &str,
        query: &str,
        method: reqwest::Method,
        body: TParam,
    ) -> Result<reqwest::Response, ReqwestError>
    where
        TParam: Serialize + Send + Sync + 'static,
    {
        let url = if query.is_empty() {
            format!("{}{}", self.url, path)
        } else {
            format!("{}{}?{}", self.url, path, query)
        };
        let request_builder = match method {
            reqwest::Method::GET => self.reqwest_client.get(&url),
            reqwest::Method::POST => self.reqwest_client.post(&url),
            _ => panic!("Unsupported method: {:?}", method),
        };

        if TypeId::of::<TParam>() == TypeId::of::<()>() {
            request_builder.json(&json!({})).send().await.map_err(ReqwestError::Error)
        } else {
            request_builder.json(&body).send().await.map_err(ReqwestError::Error)
        }
    }

    pub async fn post_no_body(&self, path: &str) -> Result<reqwest::Response, ReqwestError> {
        let url = format!("{}{}", self.url, path);
        self.reqwest_client.post(&url).send().await.map_err(ReqwestError::Error)
    }
}

#[async_trait::async_trait]
pub trait PostReqwestSender<TParam, TResponse>: Clone + Send + Sync + 'static {
    /// Sends a POST request to the devnet with the given path and body (`TParam`) and returns the
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
pub trait GetReqwestSender<TResponse>: Clone + Send + Sync + 'static {
    /// Sends a GET request to the devnet with the given path and query string and returns the
    /// response
    ///
    /// # Arguments
    /// * `path` - The path to send the request to
    /// * `query` - The query string to append to the path
    ///
    /// # Returns
    /// The response from the devnet in the form of the type `TResponse`
    /// If http status is not success it will return a `super::errors::ReqwestError`
    async fn get_json_async(
        &self,
        path: &str,
        query: Option<String>,
    ) -> Result<TResponse, ReqwestError>;
}

#[async_trait::async_trait]
impl<TResponse> GetReqwestSender<TResponse> for ReqwestClient
where
    TResponse: DeserializeOwned + 'static,
{
    async fn get_json_async(
        &self,
        path: &str,
        query: Option<String>,
    ) -> Result<TResponse, ReqwestError> {
        let response =
            self.get_response(path, &query.unwrap_or("".into()), Method::GET, ()).await?;

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
impl GetReqwestSender<HttpEmptyResponseBody> for ReqwestClient {
    async fn get_json_async(
        &self,
        path: &str,
        query: Option<String>,
    ) -> Result<HttpEmptyResponseBody, ReqwestError> {
        let response =
            self.get_response(path, &query.unwrap_or("".into()), Method::GET, ()).await?;

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

#[async_trait::async_trait]
impl<TParam, TResponse> PostReqwestSender<TParam, TResponse> for ReqwestClient
where
    TParam: Serialize + Send + Sync + 'static,
    TResponse: DeserializeOwned + 'static,
{
    async fn post_json_async(&self, path: &str, body: TParam) -> Result<TResponse, ReqwestError> {
        let response = self.get_response(path, "", reqwest::Method::POST, body).await?;

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
impl<TParam> PostReqwestSender<TParam, HttpEmptyResponseBody> for ReqwestClient
where
    TParam: Serialize + Send + Sync + 'static,
{
    async fn post_json_async(
        &self,
        path: &str,
        body: TParam,
    ) -> Result<HttpEmptyResponseBody, ReqwestError> {
        let response = self.get_response(path, "", reqwest::Method::POST, body).await?;

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
