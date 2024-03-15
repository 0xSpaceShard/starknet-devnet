use std::net::SocketAddr;
use std::str::FromStr;

use axum::routing::{get, post};
use starknet_core::starknet::starknet_config::StarknetConfig;

use crate::api::http::{endpoints as http, HttpApiHandler};
use crate::api::json_rpc::{JsonRpcHandler, OriginForwarder};
use crate::api::Api;
use crate::builder::StarknetDevnetServer;
use crate::error::ServerResult;
use crate::ServerConfig;

/// Configures an [axum::Server] that handles related JSON-RPC calls and WEB API calls via HTTP
pub fn serve_http_api_json_rpc(
    addr: SocketAddr,
    config: ServerConfig,
    api: Api,
    starknet_config: &StarknetConfig,
) -> ServerResult<StarknetDevnetServer> {
    let http = HttpApiHandler { api: api.clone() };
    let origin_caller = starknet_config
        .fork_config
        .url
        .as_ref()
        .map(|url| OriginForwarder::new(hyper::Uri::from_str(url.as_str()).unwrap()));

    let json_rpc = JsonRpcHandler { api, origin_caller };

    crate::builder::Builder::<JsonRpcHandler, HttpApiHandler>::new(addr, json_rpc, http)
        .set_config(config)
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
        .http_api_route("/fee_token", get(http::mint_token::get_fee_token))
        .http_api_route("/mint", post(http::mint_token::mint))
        .http_api_route("/fork_status", get(http::get_fork_status))
        .build(starknet_config)
}
