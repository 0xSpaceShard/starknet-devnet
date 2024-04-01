<!-- logo / title -->
<p align="center" style="margin-bottom: 0px !important">
  <img width="200" src="https://github.com/leoyoung-69/starknet-devnet-rs-logo/assets/21069052/be4f3ec2-0158-4854-9b76-7890ef8effd7" alt="Devnet-RS" align="center">

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

- [x] RPC v0.7.0
- [x] [Dump & Load](#dumping--loading)
- [x] [Mint token - Local faucet](#mint-token)
- [x] [Customizable predeployed accounts](#predeployed-contracts)
- [x] [Starknet.js test suite passes 100%](https://github.com/starknet-io/starknet.js/actions)
- [x] [Advancing time](https://0xspaceshard.github.io/starknet-devnet/docs/guide/advancing-time)
- [x] [Availability as a package (crate)](#installing-from-cratesio)
- [x] [Forking](#forking)
- [x] [L1-L2 Postman integration](https://0xspaceshard.github.io/starknet-devnet/docs/guide/postman)
- [x] [Block manipulation](https://0xspaceshard.github.io/starknet-devnet/docs/guide/blocks)
  - [x] [Aborting blocks](#abort-blocks)
  - [x] [Creating an empty block](#create-an-empty-block)

### TODO to reach feature parity with the Pythonic Devnet

- [ ] Creating blocks on demand

## Requirements

Make sure to have installed [Rust](https://www.rust-lang.org/tools/install).

The required Rust version is specified in [rust-toolchain.toml](rust-toolchain.toml) and handled automatically by `cargo`.

## Run from source

After git-cloning this repository, install and run the project with:

```
$ cargo run
```

For a more optimized and faster performance (though with a longer compilation time), run with:

```
$ cargo run --release
```

## Run as a binary

Installing and running as a binary is achievable via `cargo install`. The project can be installed from crates.io and github.com.

### Installing from crates.io

```
$ cargo install starknet-devnet
```

### Installing from github

- Use the `--locked` flag to ensure using the dependencies listed in [the lock file](/Cargo.lock)
- Preferrably familiarize yourself with the `cargo install` command ([docs](https://doc.rust-lang.org/cargo/commands/cargo-install.html#dealing-with-the-lockfile))

```
$ cargo install --git https://github.com/0xSpaceShard/starknet-devnet-rs.git --locked
```

When the installation finishes, follow the output in your terminal.

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

All of the versions published on crates.io for starknet-devnet are available as docker images, which can be used via:

```
$ docker pull shardlabs/starknet-devnet-rs:<CRATES_IO_VERSION>
```

NOTE! The latest docker image tag corresponds to the last published version in crates.io

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

Unlike Pythonic Devnet, which supported the gateway and feeder gateway API, Devnet in Rust only supports JSON-RPC. Since JSON-RPC v0.6.0, to find out which JSON-RPC version is supported by which Devnet version, check out the [releases page](https://github.com/0xspaceshard/starknet-devnet-rs/releases).

Below is the list of old RPC versions supported by Devnet, usable as git tags or branches.

- `json-rpc-v0.4.0`
- `json-rpc-v0.5.0`
- `json-rpc-v0.5.1`

These revisions should be used with `git checkout <REVISION>`.

The JSON-RPC API is reachable via `/rpc` and `/` (e.g. if spawning Devnet with default settings, these URLs have the equivalent functionality: `http://127.0.0.1:5050/rpc` and `http://127.0.0.1:5050/`)

## Predeployed contracts

Devnet predeploys a [UDC](https://docs.openzeppelin.com/contracts-cairo/0.6.1/udc), an [ERC20 (fee token)](https://docs.openzeppelin.com/contracts-cairo/0.8.1/erc20) contract and a set of predeployed funded accounts.

The set of accounts can be controlled via [CLI options](#cli-options): `--accounts <NUMBER_OF>`, `--initial-balance <WEI>`, `--seed <VALUE>`.

Choose between predeploying Cairo 0 (OpenZeppelin 0.5.1) or Cairo 1 (default; OpenZeppelin 0.8.1) accounts by using `--account-class [cairo0 | cairo1]`. Alternatively, provide a path to the [Sierra artifact](https://github.com/starkware-libs/cairo#compiling-and-running-cairo-files) of your custom account using `--account-class-custom <SIERRA_PATH>`.

The predeployment information is logged on Devnet startup. Predeployed accounts can be retrieved in JSON format by sending a `GET` request to `/predeployed_accounts` of your Devnet.

## Mint token

For now, you can consult the [Pythonic Devnet docs on minting](https://0xspaceshard.github.io/starknet-devnet/docs/guide/mint-token/), with the differences between lite minting not being supported anymore and additional support of Stark token minting declared in FRI unit. Unit is an optional parameter and when it's not specified is set to WEI by default, this behaviour can change in the next versions.

```
POST /mint
{
    "address": "0x6e3205f...",
    "amount": 500000,
    "unit": "FRI"
}
```

### Check balance

Check the balance of an address by sending a GET request to `/account_balance`. The address should be a 0x-prefixed hex string; the unit defaults to `WEI`.

```
GET /account_balance?address=<ADDRESS>&[unit=<FRI|WEI>]
```

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

- Dumping on request requires providing --dump-on mode on the startup. Example usage in `exit` mode (replace `<HOST>`, `<PORT>` and `<PATH>` with your own):

```
cargo run -- --dump-on exit --dump-path <PATH>
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

### Loading disclaimer

Dumping and loading is not guaranteed to work cross-version. I.e. if you dumped one version of Devnet, do not expect it to be loadable with a different version.
If you dumped a Devnet utilizing one class for account predeployment (e.g. the default `--account-class cairo0`), you should use the same option when loading.

## Restarting

Devnet can be restarted by making a `POST /restart` request (no body required). All of the deployed contracts (including predeployed), blocks and storage updates will be restarted to the original state, without the transactions and requests from a dump file you may have provided on startup.

If you're using [**the Hardhat plugin**](https://github.com/0xSpaceShard/starknet-hardhat-plugin#restart), restart with `starknet.devnet.restart()`.

## Blocks

A new block is generated with each new transaction, and you can create an empty block by yourself.

### Create an empty block

To create an empty block without transactions, POST a request to /create_block:

```
POST /create_block
```

Response:

```
{"block_hash": "0x115e1b390cafa7942b6ab141ab85040defe7dee9bef3bc31d8b5b3d01cc9c67"}
```

### Abort blocks

This functionality allows to simulate block abortion that can occur on mainnet.

You can abort blocks and revert transactions from the specified block to the currently latest block. Newly created blocks after the abortion will have accepted status and will continue with numbering where the last accepted block left off.

The state of Devnet will be reverted to the state of the last accepted block.

E.g. assume there are 3 accepted blocks numbered 1, 2 and 3. Upon receiving a request to abort blocks starting with block 2, the blocks numbered 2 and 3 are aborted and their transactions reverted. The state of network will be as it was in block 1. Once a new block is mined, it will be accepted and it will have number 2.

Aborted blocks can only be queried by block hash. Aborting the blocks in forking origin and already aborted blocks is not supported and results in an error.

```
POST /abort_blocks
{
    "starting_block_hash": BLOCK_HASH
}
```

Response:

```
{
    "aborted": [BLOCK_HASH_0, BLOCK_HASH_1, ...]
}
```

## Advancing time

Block timestamp can be manipulated by setting the exact time or setting the time offset. By default, timestamp methods `/set_time` and `/increase_time` generate a new block. This can be changed for `/set_time` by setting the optional parameter `generate_block` to `false`. This skips immediate new block generation, but will use the specified timestamp whenever the next block is supposed to be generated.

All values should be set in [Unix time seconds](https://en.wikipedia.org/wiki/Unix_time).

### Set time

Sets the exact time and generates a new block.

```
POST /set_time
{
    "time": TIME_IN_SECONDS
}
```

Doesn't generate a new block, but sets the exact time for the next generated block.

```
POST /set_time
{
    "time": TIME_IN_SECONDS,
    "generate_block": false
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
cargo run -- --start-time <START_TIME_IN_SECONDS>
```

## Timeout

Timeout can be passed to Devnet's HTTP server. This makes it easier to deploy and manage large contracts that take longer to execute.

```
cargo run -- --timeout <TIMEOUT>
```

## Forking

To interact with contracts deployed on mainnet or testnet, you can use the forking to simulate the origin and experiment with it locally, making no changes to the origin itself.

```
cargo run -- --fork-network <URL> [--fork-block <BLOCK_NUMBER>]
```

The value passed to `--fork-network` should be the URL to a Starknet JSON-RPC API provider. Specifying a `--fork-block` is optional; it defaults to the `"latest"` block at the time of Devnet's start-up. All calls will first try Devnet's state and then fall back to the forking block.

### Forking status

```
GET /fork_status
```

Response when Devnet is a fork of an origin:

```js
{
  "url": "https://your.origin.io",
  "block": 42 // the block from which origin was forked
}
```

Response when not forking: `{}`

### Querying old state by specifying block hash or number

With state archive capacity set to `full`, Devnet will store full state history. The default mode is `none`, where no old states are stored.

```
cargo run -- --state-archive-capacity <CAPACITY>
```

All RPC endpoints that support querying the state at an old (non-latest) block only work with state archive capacity set to `full`.

## Development

### Development - Visual Studio Code

It is highly recommended to get familiar with [Visual Studio Code Dev Containers](https://code.visualstudio.com/docs/devcontainers/create-dev-container#_dockerfile) and install [rust-analyzer](https://code.visualstudio.com/docs/languages/rust) extension.

### Development - Linter

Run the linter with:

```
./scripts/clippy_check.sh
```

### Development - Formatter

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

### Development - Unused dependencies

To check for unused dependencies, run:

```
./scripts/check_unused_deps.sh
```

If you think this reports a dependency as a false-positive (i.e. isn't unused), check [here](https://github.com/bnjbvr/cargo-machete#false-positives).

### Development - Testing

### Prerequisites

Some tests require the `anvil` command, so you need to [install Foundry](https://book.getfoundry.sh/getting-started/installation). The `anvil` command might not be usable by tests if you run them using VS Code's `Run Test` button available just above the test case. Either run tests using a shell which has foundry/anvil in `PATH`, or modify the BackgroundAnvil Command to specify `anvil` by its path on your system.

To ensure that integration tests pass, be sure to have run `cargo build --release` or `cargo run --release` prior to testing. This builds the production target used in integration tests, so spawning BackgroundDevnet won't time out.

### Test execution

Run all tests using all available CPUs with:

```
cargo test
```

The previous command might cause your testing to die along the way due to memory issues. In that case, limiting the number of jobs helps, but depends on your machine (rule of thumb: N=6):

```
cargo test --jobs <N>
```

### Development - Docker

Due to internal needs, images with arch suffix are built and pushed to Docker Hub, but this is not mentioned in the user docs as users should NOT be needing it.

This is what happens under the hood on `main`:

- build `shardlabs/starknet-devnet-rs-<COMMIT_SHA1>-amd`
- build `shardlabs/starknet-devnet-rs-<COMMIT_SHA1>-arm`
- create and push joint docker manifest called `shardlabs/starknet-devnet-rs-<COMMIT_SHA1>`
  - same for `latest`

In the image, `tini` is used to properly handle killing of dockerized Devnet with Ctrl+C

### Development - L1 / L2 (postman)

To test Starknet messaging, Devnet exposes endpoints prefixed with `postman/` which are dedicated to the messaging feature.
You can find a full guide to test the messaging feature in the [contracts/l1-l2-messaging README](./contracts/l1-l2-messaging/README.md).

Devnet exposes the following endpoints:

- `/postman/load_l1_messaging_contract`: deploys the `MockStarknetMessaging` contract on L1 (requires L1 node to be running).
- `/postman/flush`: fetches and executes L1 -> L2 messages, and sends L2 -> L1 messages (requires L1 node to be running if `dry_run` option is not used).
- `/postman/send_message_to_l2`: sends and executes a message on L2 (L1 node **not** required).
- `/postman/consume_message_from_l2`: consumes a message on L1 node from the L2 (requires L1 node to be running).

### Development - Update of OpenZeppelin contracts

Tests in devnet require an erc20 contract with the `Mintable` feature, keep in mind that before the compilation process of [cairo-contracts](https://github.com/OpenZeppelin/cairo-contracts/) you need to mark the `Mintable` check box in this [wizard](https://wizard.openzeppelin.com/cairo) and copy this implementation to `/src/presets/erc20.cairo`.

## ✏️ Contributing

We ❤️ and encourage all contributions!

[Click here](https://0xspaceshard.github.io/starknet-devnet/docs/guide/development) for the development guide.

## 🙌 Special Thanks

Special thanks to all the [contributors](https://github.com/0xSpaceShard/starknet-devnet-rs/graphs/contributors)!
