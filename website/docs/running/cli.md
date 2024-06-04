---
sidebar_position: 2.3
---

# CLI options

Check out the CLI options with:

```
$ starknet-devnet --help
```

Or if using dockerized Devnet:

```
$ docker run --rm shardlabs/starknet-devnet-rs --help
```

## Environment variables

Every CLI parameter can also be specified via an environment variable. To see the exact variable names, run:

```
$ starknet-devnet --help
```

### Docker

```
$ docker run \
    -e <VAR1>=<VALUE> \
    -e <VAR2>=<VALUE> \
    ...
    shardlabs/starknet-devnet-rs
```

## Load configuration from a file

By relying on [environment variables](#environment-variables), prepare your configuration in a file like this:

```bash
# .my-env-file

export SEED=42
export ACCOUNTS=3
...
```

Then run:

```bash
$ source .my-env-file && starknet-devnet
```

To run in a subshell and prevent environment pollution, use parentheses:

```bash
$ (source .my-env-file && starknet-devnet)
```

### Docker

To load the variables with Docker, run:

```
$ docker run --env-file .my-env-file shardlabs/starknet-devnet-rs
```
