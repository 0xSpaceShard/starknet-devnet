# starknet-devnet-rs

A local testnet for Starknet... in Rust

This repository is work in progress, please be patient. As of Starknet 0.12.0, the [Pythonic Devnet](https://github.com/0xSpaceShard/starknet-devnet) is still the way to go.

## Requirements

It is required to install the latest version of [Rust](https://www.rust-lang.org/tools/install).

## Run

Install and run the project with:

```
cargo run
```

## CLI options

Check out the CLI options with one of:

```
cargo run -- -h
cargo run -- --help
```

## Logging

By default, the logging level is INFO, but this can be changed via the `RUST_LOG` environment variable.

All logging levels: `TRACE`, `DEBUG`, `INFO`, `WARN`, `ERROR`

To specify the logging level and run Devnet on the same line, you can do:

```
RUST_LOG=<LEVEL> cargo run
```

## Mint token

For now, you can consult the [Pythonic Devnet docs on minting](https://0xspaceshard.github.io/starknet-devnet/docs/guide/mint-token/), with the difference of lite minting not being supported anymore.

## Development - Visual Studio Code

It is highly recommended to get familiar with [Visual Studio Code Dev Containers](https://code.visualstudio.com/docs/devcontainers/create-dev-container#_dockerfile) and install [rust-analyzer](https://code.visualstudio.com/docs/languages/rust) extension.

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

If you encounter an error like

```
error: toolchain 'nightly-x86_64-unknown-linux-gnu' is not installed
```

Resolve it with:

```
rustup default nightly
```

## Development - Testing

Run all tests with:

```
cargo test
```

To ensure that integration tests pass, be sure to have run `cargo build --release` or `cargo run --release` prior to that (this will build the production target that is used in these tests, so spawning Background Devnet won't time out)
