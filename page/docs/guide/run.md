---
sidebar_position: 1
---

# Run

Installing the package adds the `starknet-devnet` command.

```text
usage: starknet-devnet [-h] [-v] [--host HOST] [--port PORT] [--load-path LOAD_PATH] [--dump-path DUMP_PATH] [--dump-on DUMP_ON]
                       [--lite-mode] [--blocks-on-demand] [--accounts ACCOUNTS] [--initial-balance INITIAL_BALANCE] [--seed SEED]
                       [--hide-predeployed-accounts] [--start-time START_TIME] [--gas-price GAS_PRICE] [--allow-max-fee-zero]
                       [--timeout TIMEOUT] [--account-class ACCOUNT_CLASS] [--fork-network FORK_NETWORK] [--fork-block FORK_BLOCK]
                       [--fork-retries FORK_RETRIES] [--chain-id CHAIN_ID] [--disable-rpc-request-validation]
                       [--disable-rpc-response-validation]

Run a local instance of Starknet Devnet

optional arguments:
  -h, --help            show this help message and exit
  -v, --version         Print the version
  --host HOST           Specify the address to listen at; defaults to 127.0.0.1 (use the address the program outputs on start)
  --port PORT, -p PORT  Specify the port to listen at; defaults to 5050
  --load-path LOAD_PATH
                        Specify the path from which the state is loaded on startup
  --dump-path DUMP_PATH
                        Specify the path to dump to
  --dump-on DUMP_ON     Specify when to dump; can dump on: exit, transaction
  --lite-mode           Introduces speed-up by skipping block hash calculation - applies sequential numbering instead (0x0, 0x1, 0x2, ...).
  --blocks-on-demand    Block generation on demand via an endpoint.
  --accounts ACCOUNTS   Specify the number of accounts to be predeployed; defaults to 10
  --initial-balance INITIAL_BALANCE, -e INITIAL_BALANCE
                        Specify the initial balance of accounts to be predeployed; defaults to 1e+21
  --seed SEED           Specify the seed for randomness of accounts to be predeployed
  --hide-predeployed-accounts
                        Prevents from printing the predeployed accounts details
  --start-time START_TIME
                        Specify the start time of the genesis block in Unix time seconds
  --gas-price GAS_PRICE, -g GAS_PRICE
                        Specify the gas price in wei per gas unit; defaults to 1e+8
  --allow-max-fee-zero  Allow transactions with max fee equal to zero
  --timeout TIMEOUT, -t TIMEOUT
                        Specify the server timeout in seconds; defaults to 60
  --account-class ACCOUNT_CLASS
                        Specify the account implementation to be used for predeploying; should be a path to the compiled JSON artifact; defaults to OpenZeppelin v1
  --fork-network FORK_NETWORK
                        Specify the network to fork: can be a URL (e.g. https://alpha-mainnet.starknet.io) or network name (valid names: alpha-goerli, alpha-goerli2, alpha-mainnet)
  --fork-block FORK_BLOCK
                        Specify the block number where the --fork-network is forked; defaults to latest
  --fork-retries FORK_RETRIES
                        Specify the number of retries of failed HTTP requests sent to the network before giving up; defaults to 1
  --chain-id CHAIN_ID   Specify the chain id as one of: {MAINNET, TESTNET, TESTNET2}; defaults to TESTNET (0x534e5f474f45524c49)
  --disable-rpc-request-validation
                        Disable requests schema validation for RPC endpoints
  --disable-rpc-response-validation
                        Disable RPC schema validation for devnet responses
  --cairo-compiler-manifest CAIRO_COMPILER_MANIFEST
                        Specify the path to the manifest (Cargo.toml) of the Cairo 1.0 compiler to be used for contract recompilation; if omitted, the default x86-compatible compiler (from cairo-lang package) is used
  --sierra-compiler-path SIERRA_COMPILER_PATH
                        Specify the path to the binary executable of starknet-sierra-compile
```

You can run `starknet-devnet` in a separate shell, or you can run it in background with `starknet-devnet &`.
Check that it's alive by running the following (address and port my vary if you specified a different one with `--host` or `--port`):

```bash
curl http://127.0.0.1:5050/is_alive
```

## Run with Docker

Devnet is available as a Docker image `shardlabs/starknet-devnet` ([Docker Hub link](https://hub.docker.com/repository/docker/shardlabs/starknet-devnet)). Fetch it by running:

```bash
docker pull shardlabs/starknet-devnet:<TAG>
```

### Versions and Tags

Devnet versions, as tracked on [PyPI](https://pypi.org/project/starknet-devnet/#history), are also the tags for the corresponding images:

- `shardlabs/starknet-devnet:<VERSION>`

The latest stable version is also available as:

- `shardlabs/starknet-devnet:latest`

Commits to the `master` branch of this repository are mostly available as images tagged with their commit hash (the full 40-hex-digits SHA1 digest):

- `shardlabs/starknet-devnet:<COMMIT_HASH>`

The last commit is also a candidate for the next release, so it is available as:

- `shardlabs/starknet-devnet:next`

So far, all listed tags referred to images built for the linux/amd64 architecture. To use arm64-compatible images, append `-arm` to the tag. E.g.:

- `shardlabs/starknet-devnet:<VERSION>-arm`
- `shardlabs/starknet-devnet:latest-arm`

By appending the `-seed0` suffix, you can use images which [predeploy funded accounts](#predeployed-accounts) with `--seed 0`, thus always deploying the same set of accounts. E.g.:

- `shardlabs/starknet-devnet:<VERSION>-seed0`
- `shardlabs/starknet-devnet:latest-seed0`
- `shardlabs/starknet-devnet:next-seed0`
- `shardlabs/starknet-devnet:<VERSION>-arm-seed0`

### Container port publishing

#### Linux

If on a Linux host machine, you can use [`--network host`](https://docs.docker.com/network/host/). This way, the port used internally by the container is also available on your host machine. The `--port` option also has effect.

```text
docker run --network host shardlabs/starknet-devnet [--port <PORT>]
```

#### Mac, Windows

If not on Linux, you need to publish the container's internally used port to a desired `<PORT>` on your host machine. The internal port is `5050` by default (can be overridden with `--port`).

```text
docker run -p [HOST:]<PORT>:5050 shardlabs/starknet-devnet
```

E.g. if you want to use your host machine's `127.0.0.1:5050`, you need to run:

```text
docker run -p 127.0.0.1:5050:5050 shardlabs/starknet-devnet
```

You may ignore any address-related output logged on container startup (e.g. `Running on all addresses` or `Running on http://172.17.0.2:5050`). What you will use is what you specified with the `-p` argument.

If you don't specify the `HOST` part, the server will indeed be available on all of your host machine's addresses (localhost, local network IP, etc.), which may present a security issue if you don't want anyone from the local network to access your Devnet instance.

## Run with the Rust implementation of Cairo VM

By default, Devnet uses the [Python implementation](https://github.com/starkware-libs/cairo-lang/) of Cairo VM.

Using the Rust implementation brings improvement for Cairo-VM-intensive operations, but introduces its own overhead, so it may not be useful for simple contracts.

You can enable it by following these steps:

1. Install compilers

Make sure you have `gcc`, `g++` and [Rust](https://www.rust-lang.org/tools/install).

2. Install [cairo-rs-py](https://github.com/lambdaclass/cairo-rs-py) in the [**same environment**](https://docs.python.org/3/library/venv.html) as Devnet:

```bash
$ pip install cairo-rs-py
```

3. Set `STARKNET_DEVNET_CAIRO_VM=rust`

```bash
$ STARKNET_DEVNET_CAIRO_VM=rust starknet-devnet
```

With Docker, use `-e`:

```bash
$ docker run -it [OPTIONS] -e STARKNET_DEVNET_CAIRO_VM=rust shardlabs/starknet-devnet [ARGS]
```

To use the Python VM, **unset** the variable or set it to `python`

```bash
$ STARKNET_DEVNET_CAIRO_VM=python starknet-devnet
```
