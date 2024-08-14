---
sidebar_position: 1
---

# Intro

:::danger Difference disclaimer

- Devnet should not be used as a replacement for official testnets. After testing on Devnet, be sure to test on a testnet (alpha-sepolia)!
- The semantics of `REJECTED` and `REVERTED` status of a transaction is not the same as on the official testnet:

| Tx status  | Official testnet                                            | Devnet                                                     |
| ---------- | ----------------------------------------------------------- | ---------------------------------------------------------- |
| `REJECTED` | validation failed; not included in a block                  | not used                                                   |
| `REVERTED` | validation passed but execution failed; included in a block | validation or execution failed; not included in a block`*` |

`*`: dummy zeroes (0) in tx info for block number and tx index

:::

You may now proceed with [running Devnet](./running/install) and checking out the many features listed in the sidebar on the left.

# Limits

As mentioned [here](https://docs.starknet.io/tools/limits-and-triggers/), "Starknet currently has a number of limits in place in order to keep the network stable and optimized for the best performance." Devnet uses the limits defined on that page and, for block-level limits, values defined [here (provided by the Blockifier team)](https://github.com/0xSpaceShard/starknet-devnet-rs/blob/main/crates/starknet-devnet-core/src/utils.rs). The block-level limits are considered only when executing transactions, not when constructing the blocks themselves. I.e. if a transaction's usage of a resource exceeds its defined block-level limit, it will be reverted; but if the cumulative usage of all transactions in a block of one resource exceeds the block limit, the block will still be generated.
