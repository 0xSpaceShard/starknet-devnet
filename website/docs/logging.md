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

By default, logging of request and response data is turned off.
To see the request and/or response body, additional level have to be provided via `RUST_LOG` environment variable.
To log the request body use `REQUEST`, to log the response body use `RESPONSE`.

NOTE! that logging request and response requires at least logging level `INFO`.

The following two commands will log request and response data with log level `INFO`.
Example:

```
$ RUST_LOG="REQUEST,RESPONSE" starknet-devnet
```

```
$ RUST_LOG="REQUEST,RESPONSE,INFO" starknet-devnet
```
