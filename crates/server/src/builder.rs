use std::convert::Infallible;
use std::net::SocketAddr;
use std::time::Duration;

use axum::response::Response;
use axum::routing::{post, IntoMakeService};
use axum::{Extension, Router};
use hyper::server::conn::AddrIncoming;
use hyper::{header, Method, Request, Server};
use starknet_core::starknet::starknet_config::StarknetConfig;
use tower::Service;
use tower_http::cors::CorsLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

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
    config: Option<ServerConfig>,
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
            config: None,
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

    /// Sets additional configuration for the [`StarknetDevnetServer`]
    pub fn set_config(self, config: ServerConfig) -> Self {
        Self { config: Some(config), ..self }
    }

    /// Creates the http server - [`StarknetDevnetServer`] from all the configured routes, provided
    /// [`ServerConfig`] and all handlers that have Some value. If TJsonRpcHandler and/or
    /// THttpApiHandler are set each methods that serves the route will be able to use it.
    /// https://docs.rs/axum/latest/axum/#using-request-extensions
    pub fn build(self, starknet_config: &StarknetConfig) -> StarknetDevnetServer {
        let mut svc = self.routes;

        svc = svc
            .layer(Extension(self.json_rpc_handler))
            .layer(Extension(self.http_api_handler))
            .layer(TraceLayer::new_for_http())
            .layer(TimeoutLayer::new(Duration::from_secs(starknet_config.timeout.into())));

        if let Some(ServerConfig { allow_origin }) = self.config {
            svc = svc.layer(
                // see https://docs.rs/tower-http/latest/tower_http/cors/index.html
                // for more details
                CorsLayer::new()
                    .allow_origin(allow_origin.0)
                    .allow_headers(vec![header::CONTENT_TYPE])
                    .allow_methods(vec![Method::GET, Method::POST]),
            )
        }

        Server::bind(&self.address).serve(svc.into_make_service())
    }
}
