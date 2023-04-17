---
sidebar_position: 19
---
# Abort Blocks After

This functionality allows to simulate blocks abort that can occur on mainnet.

Abort blocks and reject transactions from the specified block to the currently latest block. Newly created blocks after the abortion will have accepted status and will continue with numbering where the last accepted block left off. Aborted blocks can only be queried by their hashes.

E.g. assume there are 3 accepted blocks numbered 1, 2 and 3 and your request aborts blocks starting with block 2. This will make blocks 2 and 3 aborted and their transactions rejected. Once a new block is mined, it will be accepted and it will have number 2.

```
POST /abort_blocks
{
    "startingBlockHash": BLOCK_HASH
}
```

Response:
```
{
    "aborted": [BLOCK_HASH_0, BLOCK_HASH_1, ...]
}
```
