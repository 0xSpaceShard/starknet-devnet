use std::time::Duration;

use axum::Router;
use axum::body::{Body, Bytes};
use axum::extract::{DefaultBodyLimit, Request};
use axum::http::{HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::{IntoMakeService, get, post};
use http_body_util::BodyExt;
use reqwest::{Method, header};
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

use crate::api::json_rpc::JsonRpcHandler;
use crate::rpc_handler::RpcHandler;
use crate::{ServerConfig, rpc_handler};
pub type StarknetDevnetServer = axum::serve::Serve<IntoMakeService<Router>, Router>;

fn json_rpc_routes<TJsonRpcHandler: RpcHandler>(json_rpc_handler: TJsonRpcHandler) -> Router {
    Router::new()
        .route("/", post(rpc_handler::handle::<TJsonRpcHandler>))
        .route("/rpc", post(rpc_handler::handle::<TJsonRpcHandler>))
        .route("/ws", get(rpc_handler::handle_socket::<TJsonRpcHandler>))
        .with_state(json_rpc_handler)
}

/// Configures an [axum::Server] that handles related JSON-RPC calls and web API calls via HTTP.
pub async fn serve_http_json_rpc(
    tcp_listener: TcpListener,
    server_config: &ServerConfig,
    json_rpc_handler: JsonRpcHandler,
) -> StarknetDevnetServer {
    let mut routes = Router::new()
        .route("/is_alive", get(|| async { "Alive!!!" })) // Only REST endpoint to simplify liveness probe
        .merge(json_rpc_routes(json_rpc_handler.clone()))
        .layer(TraceLayer::new_for_http());

    if server_config.log_response {
        routes = routes.layer(axum::middleware::from_fn(response_logging_middleware));
    };

    routes = routes
        .layer(TimeoutLayer::new(Duration::from_secs(server_config.timeout.into())))
        .layer(DefaultBodyLimit::disable())
        .layer(
            // More details: https://docs.rs/tower-http/latest/tower_http/cors/index.html
            CorsLayer::new()
                .allow_origin(HeaderValue::from_static("*"))
                .allow_headers(vec![header::CONTENT_TYPE])
                .allow_methods(vec![Method::GET, Method::POST]),
        );

    if server_config.log_request {
        routes = routes.layer(axum::middleware::from_fn(request_logging_middleware));
    }

    axum::serve(tcp_listener, routes.into_make_service())
}

async fn log_body_and_path<T>(
    body: T,
    uri_option: Option<axum::http::Uri>,
) -> Result<axum::body::Body, (StatusCode, String)>
where
    T: axum::body::HttpBody<Data = Bytes>,
    T::Error: std::fmt::Display,
{
    let bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(err) => {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, err.to_string()));
        }
    };

    if let Ok(body_str) = std::str::from_utf8(&bytes) {
        if let Some(uri) = uri_option {
            tracing::info!("{} {}", uri, body_str);
        } else {
            tracing::info!("{}", body_str);
        }
    } else {
        tracing::error!("Failed to convert body to string");
    }

    Ok(Body::from(bytes))
}

async fn request_logging_middleware(
    request: Request,
    next: Next,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let (parts, body) = request.into_parts();

    let body = log_body_and_path(body, Some(parts.uri.clone())).await?;
    Ok(next.run(Request::from_parts(parts, body)).await)
}

async fn response_logging_middleware(
    request: Request,
    next: Next,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let response = next.run(request).await;

    let (parts, body) = response.into_parts();

    let body = log_body_and_path(body, None).await?;

    let response = Response::from_parts(parts, body);
    Ok(response)
}
