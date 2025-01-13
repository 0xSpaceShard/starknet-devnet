use std::time::Duration;

use axum::body::{Body, Bytes};
use axum::extract::{DefaultBodyLimit, Request, State};
use axum::http::{HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, IntoMakeService, MethodRouter};
use axum::Router;
use http_body_util::BodyExt;
use lazy_static::lazy_static;
use reqwest::{header, Method};
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

use crate::api::http::{endpoints as http, HttpApiHandler};
use crate::api::json_rpc::JsonRpcHandler;
use crate::restrictive_mode::is_uri_path_restricted;
use crate::rpc_handler::RpcHandler;
use crate::{http_rpc_router, rpc_handler, ServerConfig};
pub type StarknetDevnetServer = axum::serve::Serve<IntoMakeService<Router>, Router>;

lazy_static! {
    static ref HTTP_API_ROUTES_WITH_HANDLERS: [(&'static str, MethodRouter<HttpApiHandler>); 5] = [
        ("/is_alive", get(http::is_alive)),
        ("/dump", post(http::dump_load::dump)),
        ("/predeployed_accounts", get(http::accounts::get_predeployed_accounts)),
        ("/account_balance", get(http::accounts::get_account_balance)),
        ("/config", get(http::get_devnet_config))
    ];
    pub static ref HTTP_API_ROUTES_WITHOUT_LEADING_SLASH: Vec<String> =
        HTTP_API_ROUTES_WITH_HANDLERS
            .iter()
            .map(|(path, _)| path)
            .chain(&[
                "/load",
                "/postman/load_l1_messaging_contract",
                "/postman/flush",
                "/postman/send_message_to_l2",
                "/postman/consume_message_from_l2",
                "/create_block",
                "/abort_blocks",
                "/restart",
                "/set_time",
                "/increase_time",
                "/mint"
            ])
            .map(|path| String::from((*path).trim_start_matches('/')))
            .collect::<Vec<String>>();
}

fn json_rpc_routes<TJsonRpcHandler: RpcHandler>(json_rpc_handler: TJsonRpcHandler) -> Router {
    Router::new()
        .route("/", post(rpc_handler::handle::<TJsonRpcHandler>))
        .route("/rpc", post(rpc_handler::handle::<TJsonRpcHandler>))
        .with_state(json_rpc_handler)
}

fn http_api_routes(http_api_handler: HttpApiHandler) -> Router {
    let mut router = Router::new();
    for (path, method_router) in HTTP_API_ROUTES_WITH_HANDLERS.iter() {
        let method_router = method_router.clone();
        router = router.route(path, method_router);
    }
    router.with_state(http_api_handler)
}

// TODO make type generic as in fn json_rpc_routes
fn converted_http_api_routes(json_rpc_handler: JsonRpcHandler) -> Router {
    http_rpc_router![
        ("/postman/load_l1_messaging_contract", devnet_postmanLoad),
        ("/postman/flush", devnet_postmanFlush),
        ("/postman/send_message_to_l2", devnet_postmanSendMessageToL2),
        ("/postman/consume_message_from_l2", devnet_postmanConsumeMessageFromL2),
        ("/load", devnet_load), // not here for dumping purposes; needs access to json_rpc_handler
        ("/create_block", devnet_createBlock),
        ("/abort_blocks", devnet_abortBlocks),
        ("/restart", devnet_restart),
        ("/set_time", devnet_setTime),
        ("/increase_time", devnet_increaseTime),
        ("/mint", devnet_mint),
    ]
    .with_state(json_rpc_handler)
}

/// Configures an [axum::Server] that handles related JSON-RPC calls and web API calls via HTTP.
pub async fn serve_http_api_json_rpc(
    tcp_listener: TcpListener,
    server_config: &ServerConfig,
    json_rpc_handler: JsonRpcHandler,
    http_handler: HttpApiHandler,
) -> StarknetDevnetServer {
    let mut routes = Router::new()
        .merge(json_rpc_routes(json_rpc_handler.clone()))
        .merge(http_api_routes(http_handler))
        .merge(converted_http_api_routes(json_rpc_handler))
        .layer(TraceLayer::new_for_http());

    if server_config.log_response {
        routes = routes.layer(axum::middleware::from_fn(response_logging_middleware));
    };

    routes = routes
        .layer(TimeoutLayer::new(Duration::from_secs(server_config.timeout.into())))
        .layer(DefaultBodyLimit::disable())
        .layer(axum::middleware::from_fn_with_state(
            server_config.request_body_size_limit,
            reject_too_big,
        ))
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

    if server_config.restricted_methods.is_some() {
        routes = routes.layer(axum::middleware::from_fn_with_state(
            server_config.clone(),
            restrictive_middleware,
        ));
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

async fn reject_too_big(
    State(payload_limit): State<usize>,
    request: Request,
    next: Next,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    fn bad_request(e: impl std::fmt::Display) -> (StatusCode, String) {
        (StatusCode::BAD_REQUEST, format!("Invalid Content-Length: {e}"))
    }

    let too_large = |content_length: usize| -> (StatusCode, String) {
        (StatusCode::PAYLOAD_TOO_LARGE, serde_json::json!({
            "error": {
                "code": -1,
                "message": format!("Request too big! Server received: {content_length} bytes; maximum (specifiable via --request-body-size-limit): {payload_limit} bytes"),
                "data": null,
            }
        }).to_string())
    };

    if let Some(content_length) = request.headers().get(header::CONTENT_LENGTH) {
        let content_length: usize =
            content_length.to_str().map_err(bad_request)?.parse().map_err(bad_request)?;

        if content_length > payload_limit {
            return Err(too_large(content_length));
        }
    }

    let response = next.run(request).await;
    Ok(response)
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

async fn restrictive_middleware(
    State(server_config): State<ServerConfig>,
    request: Request,
    next: Next,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    if let Some(restricted_paths) = &server_config.restricted_methods {
        if is_uri_path_restricted(request.uri().path(), restricted_paths) {
            return Err((StatusCode::FORBIDDEN, "Devnet is in restrictive mode".to_string()));
        }
    }
    Ok(next.run(request).await)
}
