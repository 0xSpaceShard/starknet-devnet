---
sidebar_position: 8
---

# Blocks

Devnet starts with a genesis block (with block number equal to `0` and block hash equal to `"0x0"`).

A new block is generated with each new transaction unless you opt for [creating blocks on demand](#create-a-block-on-demand).

### Create an empty block

To create an empty block without transactions, `POST` a request to `/create_block`:

```
POST /create_block
```

Response:

```
{'block_hash': '0x115e1b390cafa7942b6ab141ab85040defe7dee9bef3bc31d8b5b3d01cc9c67'}
```

### Create a block on demand

If you start Devnet with `--blocks-on-demand` CLI option, all transactions will be pending and stored in a pending block (targetable via block ID `"pending"`).
To create a block on demand, send a `POST` request to `/create_block`. This will convert the pending block to the latest block (targetable via block ID `"latest"`), giving it a block hash and a block number. All subsequent transactions will be stored to a new pending block.

In case of demanding block creation with no pending transactions, a new empty block will be generated.

The creation of the genesis block is not affected by this feature.

```
POST /create_block
```

Response:

```
{'block_hash': '0x115e1b390cafa7942b6ab141ab85040defe7dee9bef3bc31d8b5b3d01cc9c67'}
```

### Abort blocks after

This functionality allows to simulate block abortion that can occur on mainnet.

You can abort blocks and reject transactions from the specified block to the currently latest block. Newly created blocks after the abortion will have accepted status and will continue with numbering where the last accepted block left off.

E.g. assume there are 3 accepted blocks numbered 1, 2 and 3 and your request aborts blocks starting with block 2. This will make blocks 2 and 3 aborted and their transactions rejected. Once a new block is mined, it will be accepted and it will have number 2.

Aborted blocks can only be queried by their hashes. The state of Devnet will be reverted to the last accepted block state. In block-on-demand mode, the pending state will be mined and aborted. Aborting the genesis block, blocks in forking origin and already aborted blocks is not supported and results in an error.

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