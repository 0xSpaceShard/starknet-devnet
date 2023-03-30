---
sidebar_position: 1
---

# Getting Started

Let's discover **[starknet-devnet](https://github.com/Shard-Labs/starknet-devnet)**.
:::danger Take care

## ⚠️ Disclaimer ⚠️

:::

- Devnet should not be used as a replacement for Alpha testnet. After testing on Devnet, be sure to test on testnet (alpha-goerli)!
- Sending transactions with max_fee set to 0 is supported (not supported on alpha-mainnet or alpha-goerli).

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

For more running possibilities, see [this](https://shard-labs.github.io/starknet-devnet/docs/guide/run).

### Windows installation

Follow this guide: https://www.spaceshard.io/blog/starknet-devnet-windows-tutorial

### Update to a newer version

If you already have installed an older version of devnet, you will need to remove it and install the newer version:

```
pip uninstall starknet-devnet

pip install starknet-devnet
```
