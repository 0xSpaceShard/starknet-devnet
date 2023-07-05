# starknet-devnet-rs

A local testnet for Starknet... in Rust

## Requirements

It's required to install the latest version of [Rust](https://www.rust-lang.org/tools/install).

## Run

Install and run the project with:

```
cargo run
```

## Development - Visual Studio Code

It's highly recommended to get familiar with [Visual Studio Code Dev Containers](https://code.visualstudio.com/docs/devcontainers/create-dev-container#_dockerfile) and install [rust-analyzer](https://code.visualstudio.com/docs/languages/rust) extension.

## Development - Linter

Run the linter with:

```
./scripts/clippy_check.sh
```

## Development - Formatter

Run the formatter with:

```
./scripts/format.sh
```

# Starting Devnet
When starting devnet 'DEVNET_PORT' environment variable needs to be set

DEVNET_PORT=<port> cargo run

By default logging level is INFO, but this can be changed via RUST_LOG environment variable.

All logging levels: TRACE, DEBUG, INFO, WARN, ERROR

If you want to provide Log level then command looks like:

RUST_LOG=<level> DEVNET_PORT=<port> cargo run
## Things to note

1. Devnet supports only Testnet chain id.