# starknet-devnet-rs
A local testnet for Starknet... in Rust


# Starting Devnet
When starting devnet 'DEVNET_PORT' environment variable needs to be set

DEVNET_PORT=<port> cargo run

By default logging level is INFO, but this can be changed via RUST_LOG environment variable.

All logging levels: TRACE, DEBUG, INFO, WARN, ERROR