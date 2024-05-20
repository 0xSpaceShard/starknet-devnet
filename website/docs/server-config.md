# Server config

## Host and port

Specify the host and the port used by the server with `--host <ADDRESS>` and `--port <NUMBER>` CLI arguments. If running with Docker, check out the [port publishing docs](./running/docker.md#container-port-publishing).

## Logging

By default, the logging level is `INFO`, but this can be changed via the `RUST_LOG` environment variable.

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
To see the request and/or response body, additional levels can be specified via the `RUST_LOG` environment variable: `REQUEST` for request body, `RESPONSE` for response body.

NOTE! Logging request and response requires at least logging level `INFO`.

For example, the following two commands will log request and response data with log level `INFO`.

```
$ RUST_LOG="REQUEST,RESPONSE" starknet-devnet
```

```
$ RUST_LOG="REQUEST,RESPONSE,INFO" starknet-devnet
```

## Timeout

Specify the maximum amount of time an HTTP request can be served. This makes it possible to deploy and manage large contracts that take longer to execute.

```
$ starknet-devnet --timeout <SECONDS>
```

## Request body size limit

Specify the maximum size of an incoming HTTP request body. This makes it possible to deploy and manage large contracts that take up more space.

```
$ starknet-devnet --request-body-size-limit <BYTES>
```

## API

Retrieve the server config by sending a `GET` request to `/config` and extracting its `server_config` property.

```
$ curl localhost:5050/config | jq .server_config
```