use std::{convert::Infallible, net::SocketAddr};

use axum::{
    response::Response,
    routing::{post, IntoMakeService},
    Extension, Router,
};
use hyper::{header, server::conn::AddrIncoming, Method, Request, Server};
use tower::Service;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{
    rpc_handler::{self, RpcHandler},
    ServerConfig,
};

pub type StarknetDevnetServer = Server<AddrIncoming, IntoMakeService<Router>>;

pub struct Builder<JsonRpcHandler: RpcHandler, HttpApiHandler: Clone + Send + Sync + 'static> {
    address: SocketAddr,
    routes: Router,
    json_rpc_handler: Option<JsonRpcHandler>,
    http_api_handler: Option<HttpApiHandler>,
    config: Option<ServerConfig>,
}

impl<JsonRpcHandler: RpcHandler, HttpApiHandler: Clone + Send + Sync + 'static>
    Builder<JsonRpcHandler, HttpApiHandler>
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

    pub fn http_api_route<T>(self, path: &str, service: T) -> Self
    where
        T: Service<Request<hyper::Body>, Response = Response, Error = Infallible>
            + Clone
            + Send
            + 'static,
        T::Future: Send + 'static,
    {
        Self {
            routes: self.routes.route(path, service),
            ..self
        }
    }

    pub fn set_http_api_handler(self, handler: HttpApiHandler) -> Self {
        Self {
            http_api_handler: Some(handler),
            ..self
        }
    }

    pub fn json_rpc_route(self, path: &str, handler: JsonRpcHandler) -> Self {
        Self {
            routes: self
                .routes
                .route(path, post(rpc_handler::handle::<JsonRpcHandler>)),
            json_rpc_handler: Some(handler),
            ..self
        }
    }

    pub fn set_config(self, config: ServerConfig) -> Self {
        Self {
            config: Some(config),
            ..self
        }
    }

    pub fn build(self) -> StarknetDevnetServer {
        let mut svc = self.routes;

        if self.json_rpc_handler.is_some() {
            svc = svc.layer(Extension(self.json_rpc_handler.unwrap()));
        }

        if self.http_api_handler.is_some() {
            svc = svc.layer(Extension(self.http_api_handler.unwrap()));
        }

        svc = svc.layer(TraceLayer::new_for_http());

        let svc = if self.config.is_none() {
            svc
        } else {
            let ServerConfig {
                allow_origin,
                no_cors,
            } = self.config.unwrap();

            if no_cors {
                svc
            } else {
                svc.layer(
                    // see https://docs.rs/tower-http/latest/tower_http/cors/index.html
                    // for more details
                    CorsLayer::new()
                        .allow_origin(allow_origin.0)
                        .allow_headers(vec![header::CONTENT_TYPE])
                        .allow_methods(vec![Method::GET, Method::POST]),
                )
            }
        };

        Server::bind(&self.address).serve(svc.into_make_service())
    }
}
