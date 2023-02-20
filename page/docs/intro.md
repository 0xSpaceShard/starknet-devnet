---
sidebar_position: 1
---

# Getting Started

Let's discover **[starknet-devnet](https://github.com/Shard-Labs/starknet-devnet)**.
:::danger Take care

## ⚠️ Disclaimer ⚠️

:::

- Devnet should not be used as a replacement for Alpha testnet. After testing on Devnet, be sure to test on testnet (alpha-goerli)!
- JSON-RPC doesn't support calling and estimating functions at the state of an old block - only `pending` and `latest` are supported.
- Sending transactions with max_fee set to 0 is supported (not supported on alpha-mainnet or alpha-goerli).

## Requirements

Works with Python versions >=3.8 and <3.10.

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

For more running possibilities, see [this](https://shard-labs.github.io/starknet-devnet/docs/guide/run).

### Windows installation

Follow this guide: https://www.spaceshard.io/blog/starknet-devnet-windows-tutorial
