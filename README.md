<!-- logo / title -->
<p align="center" style="margin-bottom: 0px !important">
  <img width="200" src="https://github.com/0xSpaceShard/starknet-devnet-rs/assets/21069052/4791b0e4-58fc-4a44-8f87-fc0db636a5c7" alt="Devnet-RS" align="center">
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

- [x] [RPC support](#api)
- [x] [Dump & Load](#dumping--loading)
- [x] [Mint token - Local faucet](#mint-token)
- [x] [Customizable predeployed accounts](#predeployed-contracts)
- [x] [Starknet.js test suite passes 100%](https://github.com/starknet-io/starknet.js/actions)
- [x] [Lite mode](#lite-mode)
- [x] [Advancing time](https://0xspaceshard.github.io/starknet-devnet/docs/guide/advancing-time)
- [x] [Availability as a package (crate)](#install-an-executable-binary)
- [x] [Forking](#forking)
- [x] [L1-L2 Postman integration](https://0xspaceshard.github.io/starknet-devnet/docs/guide/postman)
- [x] [Block manipulation](https://0xspaceshard.github.io/starknet-devnet/docs/guide/blocks)
  - [x] [Aborting blocks](#abort-blocks)
  - [x] [Creating an empty block](#create-an-empty-block)
  - [x] [Creating blocks on demand](#creating-blocks-on-demand)

## Installation and running

There are several approaches to installing and running Devnet.

### Requirements

Any of the approaches below that mention `cargo` require you to have [installed Rust](https://www.rust-lang.org/tools/install). You might also need to install `pkg-config` and `make`.

The required Rust version is specified in [rust-toolchain.toml](rust-toolchain.toml) and handled automatically by `cargo`.

### Install an executable binary

Installing an executable binary is achievable with `cargo install` via crates.io or github.com. This approach downloads the crate, builds it in release mode and copies it to `~/.cargo/bin/`. To avoid needing to compile and wait, check the [pre-compiled binary section](#fetch-a-pre-compiled-binary-executable).

If in the past you installed [Pythonic Devnet](https://github.com/0xSpaceShard/starknet-devnet), be sure to remove it to avoid name collision of the old and the new executable - if by no other means, then by `rm $(which starknet-devnet)`.

#### Install from crates.io

```
$ cargo install starknet-devnet
```

#### Install from GitHub

- Use the `--locked` flag to ensure using the dependencies listed in [the lock file](/Cargo.lock)
- Preferably familiarize yourself with the `cargo install` command ([docs](https://doc.rust-lang.org/cargo/commands/cargo-install.html#dealing-with-the-lockfile))

```
$ cargo install --git https://github.com/0xSpaceShard/starknet-devnet-rs.git --locked
```

#### Run the installed executable

When `cargo install` finishes, follow the output in your terminal. If properly configured, you should be able to run Devnet with:

```
$ starknet-devnet
```

### Fetch a pre-compiled binary executable

If you want to save time and skip project compilation on installation, since Devnet v0.0.5, the Assets section of each [GitHub release](https://github.com/0xSpaceShard/starknet-devnet-rs/releases) contains a set of platform-specific pre-compiled binary executables. Extract and run with:

```
$ curl https://github.com/0xSpaceShard/starknet-devnet-rs/releases/download/<VERSION>/<COMPRESSED_ARCHIVE> | tar -xvzf -C <TARGET_DIR>
$ <TARGET_DIR>/starknet-devnet
```

### Run from source

After [git-cloning](https://github.com/git-guides/git-clone) this repository, running the following command will install, build and start Devnet:

```
$ cargo run
```

Specify optional CLI params like this:

```
$ cargo run -- [ARGS]
```

For a more optimized performance (though with a longer compilation time), run:

```
$ cargo run --release
```

### Run with Docker

Devnet is available as a Docker image ([Docker Hub link](https://hub.docker.com/r/shardlabs/starknet-devnet-rs/)). To download the `latest` image, run:

```text
$ docker pull shardlabs/starknet-devnet-rs
```

Supported platforms: linux/amd64 and linux/arm64 (also executable on darwin/arm64).

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
$ starknet-devnet --help
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
$ RUST_LOG=<LEVEL> starknet-devnet
```

or if using dockerized Devnet:

```
$ docker run -e RUST_LOG=<LEVEL> shardlabs/starknet-devnet-rs
```

## API

Unlike Pythonic Devnet, which supported the gateway and feeder gateway API, Devnet in Rust only supports JSON-RPC. Since JSON-RPC v0.6.0, to find out which JSON-RPC version is supported by which Devnet version, check out the [releases page](https://github.com/0xspaceshard/starknet-devnet-rs/releases).

Below is the list of old RPC versions supported by Devnet, usable as git tags or branches. They should be used with `git checkout <REVISION>`.

- `json-rpc-v0.4.0`
- `json-rpc-v0.5.0`
- `json-rpc-v0.5.1`

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

Check the balance of an address by sending a GET request to `/account_balance`. The address should be a 0x-prefixed hex string; the unit defaults to `WEI` and block_tag to `latest`.

```
GET /account_balance?address=<ADDRESS>&[unit=<FRI|WEI>]&[block_tag=<latest|pending>]
```

## Dumping & Loading

To preserve your Devnet instance for future use, these are the options:

- Dumping on exit (handles Ctrl+C, i.e. SIGINT, doesn't handle SIGKILL):

```
$ starknet-devnet --dump-on exit --dump-path <PATH>
```

- Dumping after each block:

```
$ starknet-devnet --dump-on block --dump-path <PATH>
```

- Dumping on request requires providing --dump-on mode on the startup. Example usage in `exit` mode (replace `<HOST>`, `<PORT>` and `<PATH>` with your own):

```
$ starknet-devnet --dump-on exit --dump-path <PATH>
$ curl -X POST http://<HOST>:<PORT>/dump -d '{ "path": <PATH> }' -H "Content-Type: application/json"
```

### Loading

To load a preserved Devnet instance, the options are:

- Loading on startup (note the argument name is not `--load-path` as it was in Devnet-py):

```
$ starknet-devnet --dump-path <PATH>
```

- Loading on request:

```
curl -X POST http://<HOST>:<PORT>/load -d '{ "path": <PATH> }' -H "Content-Type: application/json"
```

Currently, dumping produces a list of received transactions that is stored on disk.
Conversely, loading is implemented as the re-execution of transactions from a dump.
This means that timestamps of `StarknetBlock` will be different.

### Loading disclaimer

Dumping and loading are not guaranteed to work cross-version. I.e. if you dumped one version of Devnet, do not expect it to be loadable with a different version.

If you dumped a Devnet utilizing one class for account predeployment (e.g. `--account-class cairo0`), you should use the same option when loading. The same applies for dumping a Devnet in `--blocks-on-demand` mode.

## Restarting

Devnet can be restarted by making a `POST /restart` request (no body required). All of the deployed contracts (including predeployed), blocks and storage updates will be restarted to the original state, without the transactions and requests from a dump file you may have provided on startup.

If you're using [**the Hardhat plugin**](https://github.com/0xSpaceShard/starknet-hardhat-plugin#restart), restart with `starknet.devnet.restart()`.

## Blocks

Devnet starts with a genesis block (with a block number equal to 0). In forking mode, the genesis block number will be equal to forked block number plus one.

A new block is generated with each new transaction, and you can create an empty block by yourself.

### Creating blocks on demand

If you start Devnet with the `--blocks-on-demand` CLI option, all valid transactions will be stored in a pending block (targetable via block tag `"pending"`).

To create a block on demand, send a `POST` request to `/create_block`. This will convert the pending block to the latest block (targetable via block tag `"latest"`), giving it a block hash and a block number. All subsequent transactions will be stored in a new pending block.

In case of demanding block creation with no pending transactions, a new empty block will be generated.

The creation of the genesis block is not affected by this feature.

```
POST /create_block
```

Response:

```
{'block_hash': '0x115e1b390cafa7942b6ab141ab85040defe7dee9bef3bc31d8b5b3d01cc9c67'}
```

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

## Lite Mode

Runs Devnet in a minimal lite mode by just skipping the block hash calculation. This is useful for testing purposes when the block hash is not needed.

```
$ starknet-devnet --lite-mode
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
$ starknet-devnet --start-time <START_TIME_IN_SECONDS>
```

## Timeout

Timeout can be passed to Devnet's HTTP server. This makes it easier to deploy and manage large contracts that take longer to execute.

```
$ starknet-devnet --timeout <TIMEOUT>
```

## Forking

To interact with contracts deployed on mainnet or testnet, you can use the forking to simulate the origin and experiment with it locally, making no changes to the origin itself.

```
$ starknet-devnet --fork-network <URL> [--fork-block <BLOCK_NUMBER>]
```

The value passed to `--fork-network` should be the URL to a Starknet JSON-RPC API provider. Specifying a `--fork-block` is optional; it defaults to the `"latest"` block at the time of Devnet's start-up. All calls will first try Devnet's state and then fall back to the forking block.

## Querying old state by specifying block hash or number

With state archive capacity set to `full`, Devnet will store full state history. The default mode is `none`, where no old states are stored.

```
$ starknet-devnet --state-archive-capacity <CAPACITY>
```

All RPC endpoints that support querying the state at an old (non-latest) block only work with state archive capacity set to `full`.

## Fetch Devnet configuration

To retrieve the current configuration of Devnet, send a GET request to `/config`. Example response is attached below. It can be interpreted as a JSON mapping of CLI input parameters, both specified and default ones, with some irrelevant parameters omitted. So use `starknet-devnet --help` to better understand the meaning of each value, though keep in mind that some of the parameters have slightly modified names.

```json
{
  "seed": 4063802897,
  "total_accounts": 10,
  "account_contract_class_hash": "0x61dac032f228abef9c6626f995015233097ae253a7f72d68552db02f2971b8f",
  "predeployed_accounts_initial_balance": "1000000000000000000000",
  "start_time": null,
  "gas_price_wei": 100000000000,
  "gas_price_strk": 100000000000,
  "data_gas_price_wei": 100000000000,
  "data_gas_price_strk": 100000000000,
  "chain_id": "SN_SEPOLIA",
  "dump_on": "exit",
  "dump_path": "dump_path.json",
  "state_archive": "none",
  "fork_config": {
    "url": "http://rpc.pathfinder.equilibrium.co/integration-sepolia/rpc/v0_7",
    "block_number": 26429
  },
  "server_config": {
    "host": "127.0.0.1",
    "port": 5050,
    "timeout": 120,
    "request_body_size_limit": 2000000
  },
  "blocks_on_demand": false,
  "lite_mode": false
}
```

## Development

### Installation

Some developer scripts used in this project are written in Python 3, with dependencies specified in `scripts/requirements.txt`. You may want to [install the dependencies in a virtual environment](https://docs.python.org/3/library/venv.html#creating-virtual-environments).

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

If you think this reports a dependency as a false positive (i.e. isn't unused), check [here](https://github.com/bnjbvr/cargo-machete#false-positives).

### Development - Spelling check

To check for spelling errors in the code, run:

```
./scripts/check_spelling.sh
```

If you think this reports a false-positive, check [here](https://crates.io/crates/typos-cli#false-positives).

### Development - pre-commit

To speed up development, you can put all the previous steps (and more) in a script defined at [.git/hooks/pre-commit](https://git-scm.com/book/en/v2/Customizing-Git-Git-Hooks).

### Development - Testing

#### Prerequisites

Some tests require the `anvil` command, so you need to [install Foundry](https://book.getfoundry.sh/getting-started/installation). The `anvil` command might not be usable by tests if you run them using VS Code's `Run Test` button available just above the test case. Either run tests using a shell which has foundry/anvil in `PATH`, or modify the BackgroundAnvil Command to specify `anvil` by its path on your system.

To ensure that integration tests pass, be sure to have run `cargo build --release` or `cargo run --release` prior to testing. This builds the production target used in integration tests, so spawning BackgroundDevnet won't time out.

#### Test execution

Run all tests using all available CPUs with:

```
$ cargo test
```

The previous command might cause your testing to die along the way due to memory issues. In that case, limiting the number of jobs helps, but depends on your machine (rule of thumb: N=6):

```
$ cargo test --jobs <N>
```

#### Benchmarking

To test if your contribution presents an improvement in execution time, check out the script at `scripts/benchmark/command_stat_test.py`.

### Development - Docker

Due to internal needs, images with arch suffix are built and pushed to Docker Hub, but this is not mentioned in the user docs as users should NOT be needing it.

This is what happens under the hood on `main`:

- build `shardlabs/starknet-devnet-rs-<COMMIT_SHA1>-amd`
- build `shardlabs/starknet-devnet-rs-<COMMIT_SHA1>-arm`
- create and push joint docker manifest called `shardlabs/starknet-devnet-rs-<COMMIT_SHA1>`
  - same for `latest`

### Development - L1 / L2 (postman)

To test Starknet messaging, Devnet exposes endpoints prefixed with `postman/` which are dedicated to the messaging feature.
You can find a full guide to test the messaging feature in the [contracts/l1-l2-messaging README](./contracts/l1-l2-messaging/README.md).

Devnet exposes the following endpoints:

- `/postman/load_l1_messaging_contract`: deploys the `MockStarknetMessaging` contract on L1 (requires L1 node to be running).
- `/postman/flush`: fetches and executes L1 -> L2 messages, and sends L2 -> L1 messages (requires L1 node to be running if `dry_run` option is not used).
- `/postman/send_message_to_l2`: sends and executes a message on L2 (L1 node **not** required).
- `/postman/consume_message_from_l2`: consumes a message on L1 node from the L2 (requires L1 node to be running).

### Development - Updating OpenZeppelin contracts

Tests in devnet require an erc20 contract with the `Mintable` feature, keep in mind that before the compilation process of [cairo-contracts](https://github.com/OpenZeppelin/cairo-contracts/) you need to mark the `Mintable` check box in this [wizard](https://wizard.openzeppelin.com/cairo) and copy this implementation to `/src/presets/erc20.cairo`.

If smart contract constructor logic has changed, Devnet's predeployment logic needs to be changed, e.g. `simulate_constructor` in `crates/starknet-devnet-core/src/account.rs`.

### Development - Updating Starknet

Updating the underlying Starknet is done by updating the `blockifier` dependency. It also requires updating the `STARKNET_VERSION` constant.

### Development - Updating JSON-RPC API

Updating the RPC requires following the specification files in the [starknet-specs repository](https://github.com/starkware-libs/starknet-specs). The spec_reader testing utility requires these files to be copied into the Devnet repository. The `RPC_SPEC_VERSION` constant needs to be updated accordingly.

### Development - New Devnet version release

To release a new version, follow these steps:

1. Increment the semver in Cargo.toml of those Devnet crates that have changed. Use `scripts/check_crate_changes.sh` for this. Preferably create a separate PR for the increment, such as [this one](https://github.com/0xSpaceShard/starknet-devnet-rs/pull/398).

2. The publishing of crates and Docker images is done automatically in CI when merged into the main branch.

3. When the CI workflow is done, create a git tag of the form `vX.Y.Z`, push it and create a GitHub release with notes describing changes since the last release.

4. Attach the [binary artifacts built in CI](https://circleci.com/docs/artifacts/#artifacts-overview) to the release. Use `scripts/fetch_ci_binaries.py` to fetch all artifacts of a CI workflow.

### Development - External PRs

Read more about how to review PRs in [the guidelines](.github/CONTRIBUTING.md#review).

Our CI/CD platform (CircleCI) does not have the option to trigger the workflow on click. So once a PR is reviewed and looks like its workflow could pass, you can either accept & merge it blindly (which shall trigger the workflow on the target branch), or use the following procedure to trigger it:

```
# https://stackoverflow.com/questions/5884784/how-to-pull-remote-branch-from-somebody-elses-repo
$ git remote add <CONTRIBUTOR> git://path/to/contributors/repo.git
$ git fetch <CONTIRBUTOR>
$ git checkout -b <CONTRIBUTOR>/<BRANCH> <CONTRIBUTOR>/<BRANCH>

$ git remote set-url --push <CONTRIBUTOR> git@github.com:0xSpaceShard/starknet-devnet-rs.git
$ git push <CONTRIBUTOR> HEAD
```

## ✏️ Contributing

We ❤️ and encourage all contributions!

[Click here](.github/CONTRIBUTING.md) for the development guide.

## 🙌 Special Thanks

Special thanks to all the [contributors](https://github.com/0xSpaceShard/starknet-devnet-rs/graphs/contributors)!
