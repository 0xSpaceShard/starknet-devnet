---
sidebar_position: 18
---

# Cairo 1 support

Declaring, deploying and interacting with Cairo 1 contracts is supported in the latest version. To successfully declare, if on an x86 machine, you don't have to do anything. If on another architecture, or if you want to specify a custom version of the Cairo 1 compiler, you need to specify a local compiler for recompilation (a necessary step in the declaraion of Cairo 1 contracts). Use one of:

- `--cairo-compiler-manifest <PATH_TO_CARGO_TOML>`
- `--sierra-compiler-path <PATH_TO_SIERRA_EXECUTABLE>`

## Docker support

Devnet's Docker image has a recompiler set up internally, so Cairo 1 is supported out-of-the-box. But to use a custom compiler, you should have a statically linked executable binary sierra compiler on your host and use it like this (use absolute paths when mounting):

```
$ docker run -it \
    -p <YOUR_PORT>:5050 \
    --mount type=bind,source=<YOUR_PATH>,target=/starknet-sierra-compile \
    -it \
    shardlabs/starknet-devnet:<TAG> \
    --sierra-compiler-path /starknet-sierra-compile
```
