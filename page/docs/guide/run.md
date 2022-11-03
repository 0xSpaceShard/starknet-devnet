---
sidebar_position: 1
---

# Run

Installing the package adds the `starknet-devnet` command.

```text
usage: starknet-devnet [-h] [-v] [--host HOST] [--port PORT] [--load-path LOAD_PATH] [--dump-path DUMP_PATH] [--dump-on DUMP_ON] [--lite-mode] [--accounts ACCOUNTS]
                       [--initial-balance INITIAL_BALANCE] [--seed SEED] [--hide-predeployed-accounts] [--start-time START_TIME] [--gas-price GAS_PRICE] [--timeout TIMEOUT]
                       [--account-class ACCOUNT_CLASS]

Run a local instance of StarkNet Devnet

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
  --lite-mode           Introduces speed-up by skipping block hash and deploy transaction hash calculation - applies sequential numbering instead (0x0, 0x1, 0x2, ...).
  --accounts ACCOUNTS   Specify the number of accounts to be predeployed; defaults to 10
  --initial-balance INITIAL_BALANCE, -e INITIAL_BALANCE
                        Specify the initial balance of accounts to be predeployed; defaults to 1e+21
  --seed SEED           Specify the seed for randomness of accounts to be predeployed
  --hide-predeployed-accounts
                        Prevents from printing the predeployed accounts details
  --start-time START_TIME
                        Specify the start time of the genesis block in Unix time seconds
  --gas-price GAS_PRICE, -g GAS_PRICE
                        Specify the gas price in wei per gas unit; defaults to 1e+11
  --timeout TIMEOUT, -t TIMEOUT
                        Specify the server timeout in seconds; defaults to 60
  --account-class ACCOUNT_CLASS
                        Specify the account implementation to be used for predeploying;
                        should be a path to the compiled JSON artifact;
                        defaults to OpenZeppelin v0.5.0
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

If not on Linux, you need to publish the container's internally used port to a desired `<PORT>` on your host machine. The internal port is `5050` by default (can be overriden with `--port`).

```text
docker run -p [HOST:]<PORT>:5050 shardlabs/starknet-devnet
```

E.g. if you want to use your host machine's `127.0.0.1:5050`, you need to run:

```text
docker run -p 127.0.0.1:5050:5050 shardlabs/starknet-devnet
```

You may ignore any address-related output logged on container startup (e.g. `Running on all addresses` or `Running on http://172.17.0.2:5050`). What you will use is what you specified with the `-p` argument.

If you don't specify the `HOST` part, the server will indeed be available on all of your host machine's addresses (localhost, local network IP, etc.), which may present a security issue if you don't want anyone from the local network to access your Devnet instance.
