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

You may now proceed with [running Devnet](./running/install) and checking out some of the many features listed in the sidebar on the left.

<!-- TODO: add doc versioning -->
<!-- TODO: add examples:
  - L1-L2 - use content of contracts/README.md, add section in postman.md that mentions the example and starknet-hardhat-plugin; consider adding developer section to postman.md
 -->
 <!-- TODO: add contribution link to the page -->
