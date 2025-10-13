---
sidebar_position: 2.1
---

# Install and run

## Requirements

Any of the approaches below that mention `cargo` require you to have [installed Rust](https://www.rust-lang.org/tools/install). You might also need to install `pkg-config` and `make`.

The required Rust version is specified in `rust-toolchain.toml` in the project root and handled automatically by `cargo`.

## Install an executable binary

Installing an executable binary is achievable with `cargo install` via [crates.io](https://crates.io/) or [github.com](https://github.com). This approach downloads the crate, builds it in release mode and copies it to `~/.cargo/bin/`. To avoid needing to compile and wait, check the [pre-compiled binary section](#fetch-a-pre-compiled-binary-executable).

### Remove Pythonic Devnet

If in the past you installed [Pythonic Devnet](https://github.com/0xSpaceShard/starknet-devnet), be sure to remove it to avoid name collision of the old and the new executable - if by no other means, then by `rm $(which starknet-devnet)`.

### Install from crates.io

```
$ cargo install starknet-devnet
```

### Install from GitHub

- Use the `--locked` flag to ensure using the dependencies listed in `Cargo.lock` in the project root.
- Preferably familiarize yourself with the `cargo install` command ([docs](https://doc.rust-lang.org/cargo/commands/cargo-install.html#dealing-with-the-lockfile)).

```
$ cargo install --git https://github.com/0xSpaceShard/starknet-devnet.git --locked
```

### Install with `asdf`

- Run the following to add the `starknet-devnet` plugin

```
$ asdf plugin add starknet-devnet
```

- Show all installable versions:

```
$ asdf list-all starknet-devnet
```

- Install latest version:

```
$ asdf install starknet-devnet latest
```

- Install specific version:

```
$ asdf install starknet-devnet 0.2.0
```

Check [asdf guide](https://asdf-vm.com/guide/getting-started.html) for more instructions on how to install & manage versions.

### Install with starkup

[Starkup](https://github.com/software-mansion/starkup) is a tool for installing and managing Starknet-related tools, including starknet-devnet. To run starkup itself:

```
$ curl --proto '=https' --tlsv1.2 -sSf https://sh.starkup.sh | sh
```

Starkup uses `asdf` underneath to manage tool versions. After installation, you can use standard ASDF commands to manage starknet-devnet.

Make sure to restart your terminal or source your shell configuration file after installation to have starknet-devnet available in your PATH.

### Run the installed executable

When `cargo install` or another installation method finishes, follow the output in your terminal. If properly configured, you should be able to run Devnet with:

```
$ starknet-devnet
```

## Fetch a pre-compiled binary executable

If you want to save time and skip project compilation on installation, since Devnet v0.0.5, the Assets section of each [GitHub release](https://github.com/0xSpaceShard/starknet-devnet/releases) contains a set of platform-specific pre-compiled binary executables. Extract and run with:

```
$ curl -sSfL https://github.com/0xSpaceShard/starknet-devnet/releases/download/<VERSION>/<COMPRESSED_ARCHIVE> | tar -xvz -C <TARGET_DIR>
$ <TARGET_DIR>/starknet-devnet
```

## Run from source

To install the project from source, after [git-cloning](https://github.com/git-guides/git-clone) the [Devnet repository](https://github.com/0xSpaceShard/starknet-devnet), running the following command will install, build and start Devnet:

```
$ cargo run
```

Specify optional CLI params like this:

```
$ cargo run -- [ARGS]
```

For a more optimized performance (though with a longer compilation time), run:

```
$ cargo run --release
```
