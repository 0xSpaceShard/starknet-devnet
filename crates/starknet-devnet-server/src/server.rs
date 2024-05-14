use axum::routing::{get, post};
use starknet_core::starknet::starknet_config::StarknetConfig;
use tokio::net::TcpListener;

use crate::api::http::{endpoints as http, HttpApiHandler};
use crate::api::json_rpc::origin_forwarder::OriginForwarder;
use crate::api::json_rpc::JsonRpcHandler;
use crate::api::Api;
use crate::builder::StarknetDevnetServer;
use crate::error::ServerResult;
use crate::ServerConfig;

/// Configures an [axum::Server] that handles related JSON-RPC calls and WEB API calls via HTTP
pub fn serve_http_api_json_rpc(
    tcp_listener: TcpListener,
    api: Api,
    starknet_config: &StarknetConfig,
    server_config: &ServerConfig,
) -> ServerResult<StarknetDevnetServer> {
    let http = HttpApiHandler { api: api.clone(), server_config: server_config.clone() };
    let origin_caller = if let (Some(url), Some(block_number)) =
        (&starknet_config.fork_config.url, starknet_config.fork_config.block_number)
    {
        Some(OriginForwarder::new(url.to_string(), block_number))
    } else {
        None
    };

    let json_rpc = JsonRpcHandler { api, origin_caller };

    crate::builder::Builder::<JsonRpcHandler, HttpApiHandler>::new(tcp_listener, json_rpc, http)
        .json_rpc_route("/")
        .json_rpc_route("/rpc")
        .http_api_route("/is_alive", get(http::is_alive))
        .http_api_route("/dump", post(http::dump_load::dump))
        .http_api_route("/load", post(http::dump_load::load))
        .http_api_route("/postman/load_l1_messaging_contract", post(http::postman::postman_load))
        .http_api_route("/postman/flush", post(http::postman::postman_flush))
        .http_api_route(
            "/postman/send_message_to_l2",
            post(http::postman::postman_send_message_to_l2),
        )
        .http_api_route(
            "/postman/consume_message_from_l2",
            post(http::postman::postman_consume_message_from_l2),
        )
        .http_api_route("/create_block", post(http::blocks::create_block))
        .http_api_route("/abort_blocks", post(http::blocks::abort_blocks))
        .http_api_route("/restart", post(http::restart))
        .http_api_route("/set_time", post(http::time::set_time))
        .http_api_route("/increase_time", post(http::time::increase_time))
        .http_api_route("/predeployed_accounts", get(http::accounts::get_predeployed_accounts))
        .http_api_route("/account_balance", get(http::accounts::get_account_balance))
        .http_api_route("/mint", post(http::mint_token::mint))
        .http_api_route("/config", get(http::get_devnet_config))
        .build(server_config)
}
