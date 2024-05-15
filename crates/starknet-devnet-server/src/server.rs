use std::time::Duration;

use axum::extract::DefaultBodyLimit;
use axum::http::HeaderValue;
use axum::routing::{get, post, IntoMakeService, MethodRouter};
use axum::Router;
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
pub fn serve_http_api_json_rpc(
    tcp_listener: TcpListener,
    api: Api,
    starknet_config: &StarknetConfig,
    server_config: &ServerConfig,
) -> StarknetDevnetServer {
    let http_handler = HttpApiHandler { api: api.clone(), server_config: server_config.clone() };
    let origin_caller = if let (Some(url), Some(block_number)) =
        (&starknet_config.fork_config.url, starknet_config.fork_config.block_number)
    {
        Some(OriginForwarder::new(url.to_string(), block_number))
    } else {
        None
    };

    let json_rpc_handler = JsonRpcHandler { api, origin_caller };
    let json_rpc_routes = json_rpc_routes(json_rpc_handler);
    let http_api_routes = http_api_routes(http_handler);

    let routes = http_api_routes
        .merge(json_rpc_routes)
        .layer(TraceLayer::new_for_http())
        .layer(TimeoutLayer::new(Duration::from_secs(server_config.timeout.into())))
        .layer(DefaultBodyLimit::max(server_config.request_body_size_limit))
        .layer(
            CorsLayer::new()
                    // More details: https://docs.rs/tower-http/latest/tower_http/cors/index.html
                    .allow_origin("*".parse::<HeaderValue>().unwrap())
                    .allow_headers(vec![header::CONTENT_TYPE])
                    .allow_methods(vec![Method::GET, Method::POST]),
        );

    axum::serve(tcp_listener, routes.into_make_service())
}
