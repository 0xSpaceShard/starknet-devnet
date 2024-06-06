---
sidebar_position: 2.3
---

# CLI options

Configure your Devnet instance by specifying CLI parameters on startup.

## Help

Check out all the options with:

```
$ starknet-devnet --help
```

Or if using dockerized Devnet:

```
$ docker run --rm shardlabs/starknet-devnet-rs --help
```

## Environment variables

Every CLI option can also be specified via an environment variable:

```
$ <VAR1>=<VALUE> <VAR2>=<VALUE> starknet-devnet
```

To see the exact variable names, use [`--help`](#help).

### Precedence

If both a CLI argument and an environment variable are passed for a parameter, the CLI argument takes precedence. If none are provided, the default value is used. E.g. if running Devnet with the following command, seed value 42 will be used:

```
$ SEED=10 starknet-devnet --seed 42
```

### Docker

If using dockerized Devnet, specify the variables like this:

```
$ docker run \
    -e <VAR1>=<VALUE> \
    -e <VAR2>=<VALUE> \
    ... \
    shardlabs/starknet-devnet-rs
```

## Load configuration from a file

If providing many configuration parameters in a single command becomes cumbersome, consider loading them from a file. By relying on [environment variables](#environment-variables), prepare your configuration in a file like this:

```bash
export SEED=42
export ACCOUNTS=3
...
```

Assuming the file is called `.my-env-file`, then run:

```bash
$ source .my-env-file && starknet-devnet
```

To run in a subshell and prevent environment pollution (i.e. to unset the variables after Devnet exits), use parentheses:

```bash
$ ( source .my-env-file && starknet-devnet )
```

### Docker

To load environment variables from `.my-env-file` with Docker, remove the `export` part in each line to have the file look like this:

```
SEED=42
ACCOUNTS=3
...
```

Then run:

```
$ docker run --env-file .my-env-file shardlabs/starknet-devnet-rs
```
