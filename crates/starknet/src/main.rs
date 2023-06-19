use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use ::server::ServerConfig;

mod api;
mod server;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt::init();
    let host = IpAddr::V4(Ipv4Addr::LOCALHOST);
    let port = 8077;
    let mut addr = SocketAddr::new(host, port);
    let server = server::serve_http_api_json_rpc(addr, ServerConfig::default());
    addr = server.local_addr();
    println!("{:?}", addr);

    // spawn the server on a new task
    let serve = tokio::task::spawn(server);

    Ok(serve.await??)
}
