# starknet-devnet-rs

A local testnet for Starknet... in Rust

This repository is work in progress, please be patient. As of Starknet 0.12.2, [Pythonic Devnet](https://github.com/0xSpaceShard/starknet-devnet) is still the way to go.

## Requirements

Make sure to have installed [Rust](https://www.rust-lang.org/tools/install).

The required Rust version is specified in [rust-toolchain.toml](rust-toolchain.toml) and handled automatically by `cargo`.

## Run

After git-cloning this repository, install and run the project with:

```
$ cargo run
```

## Run with Docker

This application is available as a Docker image ([Docker Hub link](https://hub.docker.com/r/shardlabs/starknet-devnet-rs/)). To download the `latest` image, run:

```text
$ docker pull shardlabs/starknet-devnet-rs
```

Supported architectures: arm64 and amd64.

Running a container is done like this (see [port publishing](#container-port-publishing) for more info):

```text
$ docker run -p [HOST:]<PORT>:5050 shardlabs/starknet-devnet-rs [OPTIONS]
```

### Docker image tags

Commits to the `main` branch of this repository are mostly available as images tagged with their commit hash (the full 40-lowercase-hex-digits SHA1 digest):

```
$ docker pull shardlabs/starknet-devnet-rs:<COMMIT_HASH>
```

By appending the `-seed0` suffix, you can use images which [predeploy funded accounts](#predeployed-contracts) with `--seed 0`, thus always predeploying the same set of accounts:

```
$ docker pull shardlabs/starknet-devnet-rs:<VERSION>-seed0
$ docker pull shardlabs/starknet-devnet-rs:latest-seed0
```

### Container port publishing

#### Linux

If on a Linux host machine, you can use [`--network host`](https://docs.docker.com/network/host/). This way, the port used internally by the container is also available on your host machine. The `--port` option can be used (as well as other CLI options).

```text
$ docker run --network host shardlabs/starknet-devnet-rs [--port <PORT>]
```

#### Mac, Windows

If not on Linux, you need to publish the container's internally used port to a desired `<PORT>` on your host machine. The internal port is `5050` by default (probably not your concern, but can be overridden with `--port`).

```text
$ docker run -p [HOST:]<PORT>:5050 shardlabs/starknet-devnet-rs
```

E.g. if you want to use your host machine's `127.0.0.1:5050`, you need to run:

```text
$ docker run -p 127.0.0.1:5050:5050 shardlabs/starknet-devnet-rs
```

You may ignore any address-related output logged on container startup (e.g. `Starknet Devnet listening on 0.0.0.0:5050`). What you will use is what you specified with the `-p` argument.

If you don't specify the `HOST` part, the server will indeed be available on all of your host machine's addresses (localhost, local network IP, etc.), which may present a security issue if you don't want anyone from the local network to access your Devnet instance.

## CLI options

Check out the CLI options with:

```
$ cargo run -- --help
```

Or if using dockerized Devnet:

```
$ docker run --rm shardlabs/starknet-devnet-rs --help
```

## Logging

By default, the logging level is INFO, but this can be changed via the `RUST_LOG` environment variable.

All logging levels: `TRACE`, `DEBUG`, `INFO`, `WARN`, `ERROR`

To specify the logging level and run Devnet on the same line:

```
$ RUST_LOG=<LEVEL> cargo run
```

or if using dockerized Devnet:

```
$ docker run -e RUST_LOG=<LEVEL> shardlabs/starknet-devnet-rs
```

## API

Unlike Pythonic Devnet, which supported the gateway and feeder gateway API, Devnet in Rust only supports JSON-RPC, which at the time of writing this is synchronized with [specification v0.4.0](https://github.com/starkware-libs/starknet-specs/tree/v0.4.0/api).

## Predeployed contracts

Devnet predeploys a [UDC](https://docs.openzeppelin.com/contracts-cairo/0.6.1/udc), an [ERC20 (fee token)](https://docs.openzeppelin.com/contracts/3.x/api/token/erc20) contract and a set of funded accounts. The information on this is logged on Devnet startup. The set of accounts can be controlled via [CLI options](#cli-options): `--accounts`, `--initial-balance`, `--seed`.

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

## Development - Docker

Due to internal needs, images with arch suffix are built and pushed to Docker Hub, but this is not mentioned in the user docs as users should NOT be needing it.

This is what happens under the hood on `main`:

- build `shardlabs/starknet-devnet-rs-<COMMIT_SHA1>-amd`
- build `shardlabs/starknet-devnet-rs-<COMMIT_SHA1>-arm`
- create and push joint docker manifest called `shardlabs/starknet-devnet-rs-<COMMIT_SHA1>`
  - same for `latest`

In the image, `tini` is used to properly handle killing of dockerized Devnet with Ctrl+C
