---
sidebar_position: 1
---

# Run

Installing the package adds the `starknet-devnet` command.

```text
usage: starknet-devnet [-h] [-v] [--host HOST] [--port PORT]

Run a local instance of Starknet Devnet

optional arguments:
  -h, --help            show this help message and exit
  -v, --version         Print the version
  --host HOST           Specify the address to listen at; defaults to
                        127.0.0.1 (use the address the program outputs on
                        start)
  --port PORT, -p PORT  Specify the port to listen at; defaults to 5050
  --load-path LOAD_PATH
                        Specify the path from which the state is loaded on
                        startup
  --dump-path DUMP_PATH
                        Specify the path to dump to
  --dump-on DUMP_ON     Specify when to dump; can dump on: exit, transaction
  --lite-mode           Applies all lite-mode-* optimizations by disabling some features.
  --lite-mode-block-hash
                        Disables block hash calculation
  --lite-mode-deploy-hash
                        Disables deploy tx hash calculation
  --accounts ACCOUNTS   Specify the number of accounts to be predeployed;
                        defaults to 10
  --initial-balance INITIAL_BALANCE, -e INITIAL_BALANCE
                        Specify the initial balance of accounts to be
                        predeployed; defaults to 1e+21 (wei)
  --seed SEED           Specify the seed for randomness of accounts to be
                        predeployed
  --start-time START_TIME
                        Specify the start time of the genesis block in Unix
                        time seconds
  --gas-price GAS_PRICE, -g GAS_PRICE
                        Specify the gas price in wei per gas unit; defaults to
                        1e+11
  --timeout TIMEOUT, -t TIMEOUT
                        Specify the timeout for devnet server; defaults to 60 seconds
```

You can run `starknet-devnet` in a separate shell, or you can run it in background with `starknet-devnet &`.
Check that it's alive by running the following (address and port my vary if you specified a different one with `--host` or `--port`):

```bash
curl http://127.0.0.1:5050/is_alive
```

## Run with Docker

Devnet is available as a Docker image ([**shardlabs/starknet-devnet**](https://hub.docker.com/repository/docker/shardlabs/starknet-devnet)):

```bash
docker pull shardlabs/starknet-devnet:<TAG>
```

### Versions and Tags

Image tags correspond to Devnet versions as on PyPI and GitHub, with the `latest` tag used for the latest image. These images are built for linux/amd64. To use the arm64 versions, since `0.1.23` you can append `-arm` to the tag. E.g.:

- `shardlabs/starknet-devnet:0.2.10` - image for the amd64 architecture
- `shardlabs/starknet-devnet:0.2.10-arm` - image for the arm64 architecture
- `shardlabs/starknet-devnet:latest-arm`

By appending the `-seed0` suffix, you can access images which [**predeploy funded accounts**](#predeployed-accounts) with `--seed 0`, thus always deploying the same set of accounts. E.g.:

- `shardlabs/starknet-devnet:0.2.10-seed0`
- `shardlabs/starknet-devnet:latest-seed0`
- `shardlabs/starknet-devnet:0.2.10-arm-seed0`

The server inside the container listens to the port 5050, which you need to publish to a desired `<PORT>` on your host machine:

```bash
docker run -p [HOST:]<PORT>:5050 shardlabs/starknet-devnet
```

E.g. if you want to use your host machine's `127.0.0.1:5050`, you need to run:

```bash
docker run -p 127.0.0.1:5050:5050 shardlabs/starknet-devnet
```

You may ignore any address-related output logged on container startup (e.g. `Running on all addresses` or `Running on http://172.17.0.2:5050`). What you will use is what you specified with the `-p` argument.

If you don't specify the `HOST` part, the server will indeed be available on all of your host machine's addresses (localhost, local network IP, etc.), which may present a security issue if you don't want anyone from the local network to access your Devnet instance.
