use std::convert::Infallible;
use std::net::SocketAddr;
use std::time::Duration;

use axum::extract::DefaultBodyLimit;
use axum::http::{HeaderValue, Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::{post, IntoMakeService};
use axum::{Extension, Router};
use hyper::server::conn::AddrIncoming;
use hyper::{header, Method, Server};
use tower::Service;
use tower_http::cors::CorsLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

use crate::error::ServerResult;
use crate::rpc_handler::{self, RpcHandler};
use crate::ServerConfig;

/// Helper type for naming the [`Server`]
pub type StarknetDevnetServer = Server<AddrIncoming, IntoMakeService<Router>>;

/// Helper for constructing a [`Server`].
/// [`Builder`] is a convenience wrapper around [`Router`] with added support for JSON-RPC and HTTP
/// The main purpose of [`Builder`] is to provide with the essentials elements for the server to
/// run: address, routes, shared state (if any) and additional configuration
/// [`Builder`] uses 2 generic types (TJsonRpcHandler, THttpApiHandler) representing objects that
/// will be available on every http request like a shared state.
/// Take a look at https://docs.rs/axum/latest/axum/#using-request-extensions

pub struct Builder<TJsonRpcHandler: RpcHandler, THttpApiHandler: Clone + Send + Sync + 'static> {
    address: SocketAddr,
    routes: Router,
    json_rpc_handler: TJsonRpcHandler,
    http_api_handler: THttpApiHandler,
}

impl<TJsonRpcHandler: RpcHandler, THttpApiHandler: Clone + Send + Sync + 'static>
    Builder<TJsonRpcHandler, THttpApiHandler>
{
    pub fn new(
        addr: SocketAddr,
        json_rpc_handler: TJsonRpcHandler,
        http_api_handler: THttpApiHandler,
    ) -> Self {
        Builder {
            address: addr,
            routes: Router::<hyper::Body>::new(),
            json_rpc_handler,
            http_api_handler,
        }
    }

    /// Adds an HTTP endpoint to a specific route
    pub fn http_api_route<THttpMethodService>(
        self,
        path: &str,
        http_service: THttpMethodService,
    ) -> Self
    where
        THttpMethodService: Service<Request<hyper::Body>, Response = Response, Error = Infallible>
            + Clone
            + Send
            + 'static,
        THttpMethodService::Future: Send + 'static,
    {
        Self { routes: self.routes.route(path, http_service), ..self }
    }

    /// Adds the object that will be available on every HTTP request
    pub fn set_http_api_handler(self, handler: THttpApiHandler) -> Self {
        Self { http_api_handler: handler, ..self }
    }

    /// Sets the path to the JSON-RPC endpoint and adds the object that will be available on every
    /// request
    pub fn json_rpc_route(self, path: &str) -> Self {
        Self {
            routes: self.routes.route(path, post(rpc_handler::handle::<TJsonRpcHandler>)),
            ..self
        }
    }

    /// Creates the http server - [`StarknetDevnetServer`] from all the configured routes, provided
    /// [`ServerConfig`] and all handlers that have Some value. If TJsonRpcHandler and/or
    /// THttpApiHandler are set each methods that serves the route will be able to use it.
    /// https://docs.rs/axum/latest/axum/#using-request-extensions
    pub fn build(self, config: &ServerConfig) -> ServerResult<StarknetDevnetServer> {
        let mut svc = self.routes;

        svc = svc
            .layer(Extension(self.json_rpc_handler))
            .layer(Extension(self.http_api_handler))
            .layer(TraceLayer::new_for_http());

        if config.log_response {
            svc = svc.layer(axum::middleware::from_fn(response_logging_middleware));
        }

        svc = svc
            .layer(TimeoutLayer::new(Duration::from_secs(config.timeout.into())))
            .layer(DefaultBodyLimit::max(config.request_body_size_limit))
            .layer(
                CorsLayer::new()
                    // More details: https://docs.rs/tower-http/latest/tower_http/cors/index.html
                    .allow_origin("*".parse::<HeaderValue>().unwrap())
                    .allow_headers(vec![header::CONTENT_TYPE])
                    .allow_methods(vec![Method::GET, Method::POST]),
            );

        if config.log_request {
            svc = svc.layer(axum::middleware::from_fn(request_logging_middleware));
        }

        Ok(Server::try_bind(&self.address)?.serve(svc.into_make_service()))
    }
}

async fn request_logging_middleware(
    request: Request<hyper::Body>,
    next: Next<hyper::Body>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let (parts, body) = request.into_parts();

    let body = log_body(body).await?;
    Ok(next.run(Request::from_parts(parts, body)).await)
}

async fn response_logging_middleware(
    request: Request<hyper::Body>,
    next: Next<hyper::Body>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let response = next.run(request).await;

    let (parts, body) = response.into_parts();

    let body = log_body(body).await?;

    let response = Response::from_parts(parts, body);
    Ok(response)
}

async fn log_body<T>(body: T) -> Result<hyper::Body, (StatusCode, String)>
where
    T: axum::body::HttpBody,
    <T as hyper::body::HttpBody>::Error: std::fmt::Display,
{
    let bytes = hyper::body::to_bytes(body)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    if let Ok(body_str) = std::str::from_utf8(&bytes) {
        tracing::info!("{}", body_str);
    } else {
        tracing::error!("Failed to convert body to string");
    }

    Ok(hyper::body::Body::from(bytes))
}
