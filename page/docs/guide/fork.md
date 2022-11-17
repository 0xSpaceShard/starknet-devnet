---
sidebar_position: 16
---

# Fork

To interact with contracts deployed on mainnet or testnet, you can use the forking feature to copy the remote origin and experiment with it locally with no changes to the origin.

```
starknet-devnet --fork-network <NAME|URL> [--fork-block <BLOCK_NUMBER>]
```

The value of `--fork-network` can either be a network name (`alpha-goerli`, `alpha-goerli2`, or `alpha-mainnet`) or a URL (e.g. `https://alpha4.starknet.io`).

The `--fork-block` parameter is optional and its value should be the block number from which the forking is done. If none is provided, defaults to the `"latest"` block at the time of Devnet's start-up.

All calls will first try Devnet's state and then fall back to the forking block.

If you are forking another Devnet instance, keep in mind that it doesn't support polling specific blocks, but will always fall back to the currently latest block.
