---
sidebar_position: 1
---

# Intro

:::danger Difference disclaimer

- Devnet should not be used as a replacement for the official testnet. After testing on Devnet, be sure to test on testnet (alpha-sepolia)!
- Devnet does not organize state data into Merkle-Patricia tries or similar tree-like structures, so:
  - calling the `starknet_getStorageProof` RPC method shall always result in `STORAGE_PROOF_NOT_SUPPORTED`
  - block roots are set to 0
- The pre-confirmed block is equivalent to the old pending block, except that its transactions are not `ACCEPTED_ON_L2` but `PRE_CONFIRMED`.
- By default, a new block is mined for each new transactions.
  - This can be modified by directing all new transactions into a pre-confirmed block, and at some point triggering block creation.
  - Transactions in a pre-confirmed block cannot be replaced by sending a transaction with a higher free from the same account.
  - Read more [here](./blocks).
- The semantics of `REJECTED` and `REVERTED` status of a transaction is not the same as on the official testnet:

| Tx status  | Official testnet                                            | Devnet                                                     |
| ---------- | ----------------------------------------------------------- | ---------------------------------------------------------- |
| `REJECTED` | validation failed; not included in a block                  | not used                                                   |
| `REVERTED` | validation passed but execution failed; included in a block | validation or execution failed; not included in a block`*` |

`*`: dummy zeroes (0) in tx info for block number and tx index

:::

You may now proceed with [running Devnet](./running/install) and checking out the multitude of features listed in the sidebar on the left.

# Limits

As mentioned [here](https://docs.starknet.io/tools/limits-and-triggers/), "Starknet currently has a number of limits in place in order to keep the network stable and optimized for the best performance." Devnet uses the limits defined on that page and, for block-level limits, values defined [here (provided by the Blockifier team)](https://github.com/0xSpaceShard/starknet-devnet/blob/main/crates/starknet-devnet-core/src/utils.rs). The block-level limits are considered only when executing transactions, not when constructing the blocks themselves. I.e. if a transaction's usage of a resource exceeds its defined block-level limit, it will be reverted; but if the cumulative usage of all transactions in a block of one resource exceeds the block limit, the block will still be generated.
