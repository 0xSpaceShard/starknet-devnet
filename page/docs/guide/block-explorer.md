---
sidebar_position: 7
---
# Block explorer

A local block explorer (Voyager), as noted [**here**](https://voyager.online/local-version/), apparently cannot be set up to work with Devnet. Read more in [**this issue**](https://github.com/Shard-Labs/starknet-devnet/issues/60).

## Blocks

Devnet starts with a genesis block (with block number equal to `0` and block hash equal to `"0x0"`).

A new block is generated with each new transaction. There is no pending block.

To create an empty block without transactions, `POST` a request to `/create_block`:

```
POST /create_block
```

Response:

```
{
    "transactions": [],
    "parent_block_hash": "0x0",
    "timestamp": 1659457385,
    "state_root": "004bee3ee...",
    "gas_price": "0x174876e800",
    "sequencer_address": "0x4bbfb0d1aa...",
    "transaction_receipts": [],
    "starknet_version": "0.9.1",
    "block_hash": "0x1",
    "block_number": 1,
    "status": "ACCEPTED_ON_L2"
}
```
