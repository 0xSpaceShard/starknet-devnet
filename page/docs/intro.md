---
sidebar_position: 1
---

# Getting Started

Let's discover **[starknet-devnet](https://github.com/0xSpaceShard/starknet-devnet)**.

:::danger Migration

## Migrating to Rust

This Python implementation of Devnet is **DEPRECATED** and being replaced with a [Rust implementation](https://github.com/0xSpaceShard/starknet-devnet-rs).

:::

:::danger Take care

## ⚠️ Disclaimer ⚠️

:::

- Devnet should not be used as a replacement for Alpha testnet. After testing on Devnet, be sure to test on testnet (alpha-goerli)!
- Sending transactions with max_fee set to 0 is supported (not supported on alpha-mainnet or alpha-goerli).
- The semantics of `REJECTED` and `REVERTED` status of a transaction is not the same as on the official testnet:

| Tx status  | Official testnet                                            | Devnet                                                     |
| ---------- | ----------------------------------------------------------- | ---------------------------------------------------------- |
| `REJECTED` | validation failed; not included in a block                  | not used                                                   |
| `REVERTED` | validation passed but execution failed; included in a block | validation or execution failed; not included in a block`*` |

`*`: dummy zeroes (0) in tx info for block number and tx index

## Requirements

Works with Python versions >=3.9 and <3.10.

On Ubuntu/Debian, first run:

```bash
$ sudo apt install -y libgmp3-dev
```

On Mac, you can use `brew`:

```bash
$ brew install gmp
```

## Install

```bash
$ pip install starknet-devnet
```

## Run

```
$ starknet-devnet
```

For more running possibilities, see [this](https://0xspaceshard.github.io/starknet-devnet/docs/guide/run).

### Windows installation

Follow this guide: https://www.spaceshard.io/blog/starknet-devnet-windows-tutorial

### Upgrade to a newer version

```bash
$ pip install --upgrade starknet-devnet
```
