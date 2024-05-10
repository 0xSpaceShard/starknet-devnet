# Logging

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
