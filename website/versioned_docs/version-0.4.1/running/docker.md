---
sidebar_position: 2.2
---

# Run with Docker

Devnet is available as a Docker image ([Docker Hub link](https://hub.docker.com/r/shardlabs/starknet-devnet-rs/)). To download the `latest` image, run:

```text
$ docker pull shardlabs/starknet-devnet-rs
```

Supported platforms: linux/amd64 and linux/arm64 (also executable on darwin/arm64).

Running a container is done like this (see [port publishing](#container-port-publishing) for more info):

```text
$ docker run -p [HOST:]<PORT>:5050 shardlabs/starknet-devnet-rs [OPTIONS]
```

## Docker image tags

All of the versions published on crates.io for starknet-devnet are available as docker images, which can be used via:

```
$ docker pull shardlabs/starknet-devnet-rs:<CRATES_IO_VERSION>
```

:::note

The `latest` docker image tag corresponds to the last published version on crates.io.

:::

### Accessing Devnet fixes before release

Commits to the `main` branch of this repository are mostly available as images tagged with their commit hash (the full 40-lowercase-hex-digits SHA1 digest):

```
$ docker pull shardlabs/starknet-devnet-rs:<COMMIT_HASH>
```

If a fix has been merged into the `main` branch of Devnet's code repository, you can access it before its inclusion in an official release. Just inspect the [`main` commit list](https://github.com/0xSpaceShard/starknet-devnet/commits/main) and copy the full SHA digest of the commit containing the fix (or preferably the latest commit) which has a green check ✔️ symbol next to the commit date (which indicates the image indeed exists). Some revisions may not have a corresponding Docker image, but these are not supposed to be bugfixes.

### Zero-seeded set of accounts

By appending the `-seed0` suffix, you can use images which [predeploy funded accounts](../predeployed) with `--seed 0`, thus always predeploying the same set of accounts:

```
$ docker pull shardlabs/starknet-devnet-rs:<VERSION>-seed0
$ docker pull shardlabs/starknet-devnet-rs:latest-seed0
```

## Container port publishing

### Linux

If on a Linux host machine, you can use [`--network host`](https://docs.docker.com/network/host/). This way, the port used internally by the container is also available on your host machine. The `--port` option can be used (as well as other CLI options).

```text
$ docker run --network host shardlabs/starknet-devnet-rs [--port <PORT>]
```

### Mac and Windows

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

## Development note

Due to internal needs, images with arch suffix are built and pushed to Docker Hub, but this is not mentioned in the user docs as users should NOT be needing it.

This is what happens under the hood on `main`:

- build `shardlabs/starknet-devnet-rs-<COMMIT_SHA1>-amd`
- build `shardlabs/starknet-devnet-rs-<COMMIT_SHA1>-arm`
- create and push joint docker manifest called `shardlabs/starknet-devnet-rs-<COMMIT_SHA1>`
  - same for `latest`
