use std::{convert::Infallible, net::SocketAddr, time::Duration};

use axum::{
    response::Response,
    routing::{post, IntoMakeService},
    Extension, Router,
};
use hyper::{header, server::conn::AddrIncoming, Method, Request, Server};
use tower::Service;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tower_http::timeout::TimeoutLayer;
use starknet_core::StarknetConfig;

use crate::{
    rpc_handler::{self, RpcHandler},
    ServerConfig,
};

/// Helper type for naming the [`Server`]
pub type StarknetDevnetServer = Server<AddrIncoming, IntoMakeService<Router>>;

/// Helper for constructing a [`Server`].
/// [`Builder`] is a convenience wrapper around [`Router`] with added support for JSON-RPC and HTTP
/// The main purpose of [`Builder`] is to provide with the essentials elements for the server to run:
/// address, routes, shared state (if any) and additional configuration
/// [`Builder`] uses 2 generic types (TJsonRpcHandler, THttpApiHandler) representing objects that will
/// be available on every http request like a shared state.
/// Take a look at https://docs.rs/axum/latest/axum/#using-request-extensions

pub struct Builder<TJsonRpcHandler: RpcHandler, THttpApiHandler: Clone + Send + Sync + 'static> {
    address: SocketAddr,
    routes: Router,
    json_rpc_handler: Option<TJsonRpcHandler>,
    http_api_handler: Option<THttpApiHandler>,
    config: Option<ServerConfig>,
}

impl<TJsonRpcHandler: RpcHandler, THttpApiHandler: Clone + Send + Sync + 'static>
    Builder<TJsonRpcHandler, THttpApiHandler>
{
    pub fn new(addr: SocketAddr) -> Self {
        Builder {
            address: addr,
            routes: Router::<hyper::Body>::new(),
            json_rpc_handler: None,
            http_api_handler: None,
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
        Self { http_api_handler: Some(handler), ..self }
    }

    /// Sets the path to the JSON-RPC endpoint and adds the object that will be available on every request
    pub fn json_rpc_route(self, path: &str, handler: TJsonRpcHandler) -> Self {
        Self {
            routes: self.routes.route(path, post(rpc_handler::handle::<TJsonRpcHandler>)),
            json_rpc_handler: Some(handler),
            ..self
        }
    }

    /// Sets additional configuration for the [`StarknetDevnetServer`]
    pub fn set_config(self, config: ServerConfig) -> Self {
        Self { config: Some(config), ..self }
    }

    /// Creates the http server - [`StarknetDevnetServer`] from all the configured routes, provided [`ServerConfig`]
    /// and all handlers that have Some value. If TJsonRpcHandler and/or THttpApiHandler are set
    /// each methods that serves the route will be able to use it.
    /// https://docs.rs/axum/latest/axum/#using-request-extensions
    pub fn build(self, starknet_config: &StarknetConfig) -> StarknetDevnetServer {
        let mut svc = self.routes;

        if self.json_rpc_handler.is_some() {
            svc = svc.layer(Extension(self.json_rpc_handler.unwrap()));
        }

        if self.http_api_handler.is_some() {
            svc = svc.layer(Extension(self.http_api_handler.unwrap()));
        }

        svc = svc.layer(TraceLayer::new_for_http())      
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
