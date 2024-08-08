use std::time::Duration;

use axum::body::{Body, Bytes};
use axum::extract::{DefaultBodyLimit, Request};
use axum::http::{HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, IntoMakeService};
use axum::Router;
use http_body_util::BodyExt;
use reqwest::{header, Method};
use starknet_core::starknet::starknet_config::StarknetConfig;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

use crate::api::http::{endpoints as http, HttpApiHandler};
use crate::api::json_rpc::origin_forwarder::OriginForwarder;
use crate::api::json_rpc::JsonRpcHandler;
use crate::api::Api;
use crate::dump::DumpEvent;
use crate::rpc_handler::RpcHandler;
use crate::{rpc_handler, ServerConfig};
pub type StarknetDevnetServer = axum::serve::Serve<IntoMakeService<Router>, Router>;

fn json_rpc_routes<TJsonRpcHandler: RpcHandler>(json_rpc_handler: TJsonRpcHandler) -> Router {
    Router::new()
        .route("/", post(rpc_handler::handle::<TJsonRpcHandler>))
        .route("/rpc", post(rpc_handler::handle::<TJsonRpcHandler>))
        .with_state(json_rpc_handler)
}

fn http_api_routes(http_api_handler: HttpApiHandler) -> Router {
    Router::new()
        .route("/is_alive", get(http::is_alive))
        .route("/dump", post(http::dump_load::dump))
        .route("/load", post(http::dump_load::load))
        .route("/postman/load_l1_messaging_contract", post(http::postman::postman_load))
        .route("/postman/flush", post(http::postman::postman_flush))
        .route("/postman/send_message_to_l2", post(http::postman::postman_send_message_to_l2))
        .route(
            "/postman/consume_message_from_l2",
            post(http::postman::postman_consume_message_from_l2),
        )
        .route("/create_block", post(http::blocks::create_block))
        .route("/abort_blocks", post(http::blocks::abort_blocks))
        .route("/restart", post(http::restart))
        .route("/set_time", post(http::time::set_time))
        .route("/increase_time", post(http::time::increase_time))
        .route("/predeployed_accounts", get(http::accounts::get_predeployed_accounts))
        .route("/account_balance", get(http::accounts::get_account_balance))
        .route("/mint", post(http::mint_token::mint))
        .route("/config", get(http::get_devnet_config))
        .with_state(http_api_handler)
}

/// Configures an [axum::Server] that handles related JSON-RPC calls and WEB API calls via HTTP
pub async fn serve_http_api_json_rpc(
    tcp_listener: TcpListener,
    api: Api,
    starknet_config: &StarknetConfig,
    server_config: &ServerConfig,
    loadable_events: &[DumpEvent],
) -> Result<StarknetDevnetServer, anyhow::Error> {
    let http_handler = HttpApiHandler { api: api.clone(), server_config: server_config.clone() };
    let origin_caller = if let (Some(url), Some(block_number)) =
        (&starknet_config.fork_config.url, starknet_config.fork_config.block_number)
    {
        Some(OriginForwarder::new(url.to_string(), block_number))
    } else {
        None
    };

    let json_rpc_handler = JsonRpcHandler {
        api,
        origin_caller,
        starknet_config: starknet_config.clone(),
        server_config: server_config.clone(),
    };

    json_rpc_handler
        .re_execute(loadable_events)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to re-execute dumped Devnet: {e}"))?;

    let json_rpc_routes = json_rpc_routes(json_rpc_handler);
    let http_api_routes =
        http_api_routes(http_handler).route_layer(axum::middleware::from_fn(convert_to_rpc));

    let mut routes = Router::new()
        .merge(http_api_routes)
        .merge(json_rpc_routes)
        .layer(TraceLayer::new_for_http());

    if server_config.log_response {
        routes = routes.layer(axum::middleware::from_fn(response_logging_middleware));
    };

    routes = routes
        .layer(TimeoutLayer::new(Duration::from_secs(server_config.timeout.into())))
        .layer(DefaultBodyLimit::max(server_config.request_body_size_limit))
        .layer(
            // More details: https://docs.rs/tower-http/latest/tower_http/cors/index.html
            CorsLayer::new()
                .allow_origin("*".parse::<HeaderValue>().unwrap())
                .allow_headers(vec![header::CONTENT_TYPE])
                .allow_methods(vec![Method::GET, Method::POST]),
        );

    if server_config.log_request {
        routes = routes.layer(axum::middleware::from_fn(request_logging_middleware));
    }

    Ok(axum::serve(tcp_listener, routes.into_make_service()))
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

async fn body_as_json_value(body: Body) -> Result<serde_json::Value, (StatusCode, String)> {
    let body_bytes = body
        .collect()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .to_bytes();

    if body_bytes.is_empty() {
        // TODO this case was added when I didn't know that the redirection was unsuccessful and
        // that the dummy response from load_impl was being returned
        Ok(serde_json::json!({}))
    } else {
        serde_json::from_slice(&body_bytes)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
    }
}

fn json_value_to_body(value: &serde_json::Value) -> Result<Body, (StatusCode, String)> {
    let bytes = serde_json::to_vec(value)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Body::from(bytes))
}

async fn convert_to_rpc(
    mut request: Request,
    next: Next,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // TODO check if method is POST?
    // TODO define mapping outside
    let rpc_method = match request.uri().to_string().as_str() {
        "/dump" => Some("devnet_dump"),
        "/load" => Some("devnet_load"),
        "/postman/load_l1_messaging_contract" => Some("devnet_postmanLoad"),
        "/postman/flush" => Some("devnet_postmanFlush"),
        "/postman/send_message_to_l2" => Some("devnet_postmanSendMessageToL2"),
        "/postman/consume_message_from_l2" => Some("devnet_postmanConsumeMessageFromL2"),
        "/create_block" => Some("devnet_createBlock"),
        "/abort_blocks" => Some("devnet_abortBlocks"),
        "/restart" => Some("devnet_restart"),
        "/set_time" => Some("devnet_setTime"),
        "/increase_time" => Some("devnet_increaseTime"),
        "/predeployed_accounts" => Some("devnet_getPredeployedAccounts"),
        "/account_balance" => Some("devnet_getAccountBalance"),
        "/mint" => Some("devnet_mint"),
        "/config" => Some("devnet_config"),
        _ => None,
    };

    if let Some(rpc_method) = rpc_method {
        let rpc_params = body_as_json_value(request.into_body()).await?;
        let new_body = json_value_to_body(&serde_json::json!({
            "method": rpc_method,
            "jsonrpc": "2.0",
            "id": "1",
            "params": rpc_params,
        }))?;
        request = Request::builder()
            .uri("/rpc")
            .header("content-type", "application/json")
            .header("accept", "application/json")
            .body(new_body)
            .unwrap();
    }

    let mut response = next.run(request).await;

    if rpc_method.is_some() {
        // TODO perhaps only modify response if status 200?
        let (parts, mut body) = response.into_parts();

        let rpc_resp = body_as_json_value(body).await?;
        let non_rpc_resp = if let Some(result) = rpc_resp.get("result") {
            result
        } else if let Some(error) = rpc_resp.get("error") {
            error
        } else {
            &rpc_resp
        };

        body = json_value_to_body(non_rpc_resp)?;
        response = Response::from_parts(parts, body);
    }

    Ok(response)
}
