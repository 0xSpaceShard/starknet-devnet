<!-- logo / title -->
<p align="center" style="margin-bottom: 0px !important">
  <img width="200" src="https://user-images.githubusercontent.com/2848732/193076972-da6fa36e-11f7-4cb3-aa29-673224f8576d.png" alt="Devnet-RS" align="center">
</p>

<h1 align="center" style="margin-top: 12px !important">Starknet Devnet RS</h1>

<p align="center" dir="auto">
  <a href="https://hub.docker.com/r/shardlabs/starknet-devnet-rs/tags" target="_blank">
    <img src="https://img.shields.io/badge/dockerhub-images-important.svg?logo=Docker" style="max-width: 100%;">
  </a>
  <a href="https://starkware.co/" target="_blank">
    <img src="https://img.shields.io/badge/powered_by-StarkWare-navy" style="max-width: 100%;">
  </a>
</p>

A local testnet for Starknet... in Rust!

This repository is work in progress, please be patient. Please check below the status of features compared with the [Pythonic Devnet](https://github.com/0xSpaceShard/starknet-devnet):

### Supported Features

- [x] RPC v0.4.0
- [x] [Dump & Load](https://github.com/0xSpaceShard/starknet-devnet-rs#dumping--loading)
- [x] [Mint token - Local faucet](https://0xspaceshard.github.io/starknet-devnet/docs/guide/mint-token)
- [x] [Customizable predeployed accounts](#predeployed-contracts)
- [x] Starknet.js test suite passes 100%
- [x] [Advancing time](https://0xspaceshard.github.io/starknet-devnet/docs/guide/advancing-time)

### TODO

- [x] RPC v0.5.0

### TODO to reach feature parity with the Pythonic Devnet

- [ ] Availability as a package (crate)
- [ ] [Forking](https://0xspaceshard.github.io/starknet-devnet/docs/guide/fork)
- [ ] [L1-L2 Postman integration](https://0xspaceshard.github.io/starknet-devnet/docs/guide/postman)
- [ ] [Block manipulation](https://0xspaceshard.github.io/starknet-devnet/docs/guide/blocks)
  - [x] Create an empty block

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

The JSON-RPC API is reachable via `/rpc` and `/` (e.g. if spawning Devnet with default settings, these URLs have the equivalent functionality: `http://127.0.0.1:5050/rpc` and `http://127.0.0.1:5050/`)

> **Note:**
>
> Out of Starknet **trace** API RPC methods, only `starknet_simulateTransactions` is supported.

## Predeployed contracts

Devnet predeploys a [UDC](https://docs.openzeppelin.com/contracts-cairo/0.6.1/udc), an [ERC20 (fee token)](https://docs.openzeppelin.com/contracts/3.x/api/token/erc20) contract and a set of predeployed funded accounts.

The set of accounts can be controlled via [CLI options](#cli-options): `--accounts <NUMBER_OF>`, `--initial-balance <WEI>`, `--seed <VALUE>`.

Choose between predeploying Cairo 0 (OpenZeppelin 0.5.1) or Cairo 1 (OpenZeppelin 0.7.0) accounts by using `--account-class [cairo0 | cairo1]`. Alternatively, provide a path to the [Sierra artifact](https://github.com/starkware-libs/cairo#compiling-and-running-cairo-files) of your custom account using `--account-class-custom <SIERRA_PATH>`.

The predeployment information is logged on Devnet startup. Predeployed accounts can be retrieved in JSON format by sending a `GET` request to `/predeployed_accounts` of your Devnet.

## Mint token

For now, you can consult the [Pythonic Devnet docs on minting](https://0xspaceshard.github.io/starknet-devnet/docs/guide/mint-token/), with the difference of lite minting not being supported anymore.

## Dumping & Loading

To preserve your Devnet instance for future use, these are the options:

- Dumping on exit (handles Ctrl+C, i.e. SIGINT, doesn't handle SIGKILL):

```
cargo run -- --dump-on exit --dump-path <PATH>
```

- Dumping after each transaction:

```
cargo run -- --dump-on transaction --dump-path <PATH>
```

- Dumping on request (replace <HOST>, <PORT> and <PATH> with your own):

```
curl -X POST http://<HOST>:<PORT>/dump -d '{ "path": <PATH> }' -H "Content-Type: application/json"
```

### Loading

To load a preserved Devnet instance, the options are:

- Loading on startup (note the argument name is not `--load-path` as it was in Devnet-py):

```
cargo run -- --dump-path <PATH>
```

- Loading on request:

```
curl -X POST http://<HOST>:<PORT>/load -d '{ "path": <PATH> }' -H "Content-Type: application/json"
```

Currently, dumping produces a list of received transactions that is stored on disk.
Conversely, loading is implemented as the re-execution of transactions from a dump.
This means that timestamps of `StarknetBlock` will be different.

### Restarting

Devnet can be restarted by making a `POST /restart` request (no body required). All of the deployed contracts (including predeployed), blocks and storage updates will be restarted to the original state, without the transactions and requests from a dump file you may have provided on startup.

If you're using [**the Hardhat plugin**](https://github.com/0xSpaceShard/starknet-hardhat-plugin#restart), restart with `starknet.devnet.restart()`.

### Cross-version disclaimer

Dumping and loading is not guaranteed to work cross-version. I.e. if you dumped one version of Devnet, do not expect it to be loadable with a different version.

## Blocks

A new block is generated with each new transaction, and you can create an empty block by yourself.

### Create an empty block

To create an empty block without transactions, POST a request to /create_block:

POST /create_block

Response:

{'block_hash': '0x115e1b390cafa7942b6ab141ab85040defe7dee9bef3bc31d8b5b3d01cc9c67'}

## Advancing time

Block timestamp can be manipulated by setting the exact time or setting the time offset. Timestamps methods `/set_time` and `/increase_time` will generate a new block. All values should be set in Unix time seconds [Unix time seconds](https://en.wikipedia.org/wiki/Unix_time).

### Set time

Sets the exact time and generates a new block.

```
POST /set_time
{
    "time": TIME_IN_SECONDS
}
```

Warning: block time can be set in the past which might lead to unexpected behavior!

### Increase time

Increases the block timestamp by the provided amount and generates a new block. All subsequent blocks will keep this increment.

```
POST /increase_time
{
    "time": TIME_IN_SECONDS
}
```

### Start time arg

Devnet can be started with the `--start-time` argument, where `START_TIME_IN_SECONDS` should be greater than 0.
```
cargo run -- --start-time START_TIME_IN_SECONDS
```

### Timeout

Timeout can be passed to Devnet's HTTP server. This makes it easier to deploy and manage large contracts that take longer to execute.
```
cargo run -- --timeout TIMEOUT
```

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

To ensure that integration tests pass, be sure to have run `cargo build --release` or `cargo run --release` prior to testing. This builds the production target used in integration tests, so spawning Background Devnet won't time out.

Run all tests using all available CPUs with:

```
cargo test
```

The previous command might cause your testing to die along the way due to memory issues. In that case, limiting the number of jobs helps, but depends on your machine (rule of thumb: N=6):

```
cargo test --jobs <N>
```

## Development - Docker

Due to internal needs, images with arch suffix are built and pushed to Docker Hub, but this is not mentioned in the user docs as users should NOT be needing it.

This is what happens under the hood on `main`:

- build `shardlabs/starknet-devnet-rs-<COMMIT_SHA1>-amd`
- build `shardlabs/starknet-devnet-rs-<COMMIT_SHA1>-arm`
- create and push joint docker manifest called `shardlabs/starknet-devnet-rs-<COMMIT_SHA1>`
  - same for `latest`

In the image, `tini` is used to properly handle killing of dockerized Devnet with Ctrl+C

## ✏️ Contributing

We ❤️ and encourage all contributions!

[Click here](https://0xspaceshard.github.io/starknet-devnet/docs/guide/development) for the development guide.

## 🙌 Special Thanks

Special thanks to all the [contributors](https://github.com/0xSpaceShard/starknet-devnet-rs/graphs/contributors)!
