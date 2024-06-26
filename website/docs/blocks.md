# Blocks

Devnet starts with a genesis block (with a block number equal to 0). In forking mode, the genesis block number will be equal to the forked block number plus one.

By default, a new block is generated with each new transaction, but you can also [create an empty block by yourself](#create-an-empty-block).

## Creating blocks on demand

If you start Devnet with the `--block-generation-on demand` CLI option, you will enable the possibility to store more than one transaction in the pending block (targetable via block tag `"pending"`).

Once you've added the desired transactions into the pending block, you can send a `POST` request to `/create_block` or `JSON-RPC` request with method name `devnet_createBlock`. This will convert the pending block to the latest block (targetable via block tag `"latest"`), giving it a block hash and a block number. All subsequent transactions will be stored in a new pending block.

In case of demanding block creation with no pending transactions, a new empty block will be generated.

The creation of the genesis block is not affected by this feature.

```
POST /create_block
```

```
JSON-RPC
{
    "jsonrpc": "2.0",
    "id": "1",
    "method": "devnet_createBlock"
}
```

Response:

```
{'block_hash': '0x115e1b390cafa7942b6ab141ab85040defe7dee9bef3bc31d8b5b3d01cc9c67'}
```

## Automatic periodic block creation

If started with the `--block-generation-on <INTERVAL>` CLI option, Devnet will behave as in `demand` mode but new blocks will be mined automatically every `<INTERVAL>` seconds. Considering this example of spawning Devnet at moment `t`:

```bash
# t
$ starknet-devnet --block-generation-on 10

# t + 1s
# user: send tx1

# t + 4s
# user: send tx2

# t + 10s
# Devnet: block automatically generated, contains tx1 and tx2

# t + 12s
# user: send tx3

# t + 14s
# user: invoke empty block creation
# Devnet: generated block contains tx3

# t + 20s
# Devnet: block automatically generated, contains no txs (manual creation did not restart the counter)
```

## Create an empty block

To create an empty block without transactions, `POST` a request:

```
JSON-RPC
{
    "jsonrpc": "2.0",
    "id": "1",
    "method": "devnet_createBlock"
}
```

Response:

```
{"block_hash": "0x115e1b390cafa7942b6ab141ab85040defe7dee9bef3bc31d8b5b3d01cc9c67"}
```

## Abort blocks

This functionality allows simulating block abortion that can occur on mainnet. It is supported in the `--state-archive-capacity full` mode.

You can abort blocks and revert transactions from the specified block to the currently latest block. Newly created blocks after the abortion will have accepted status and will continue with numbering where the last accepted block left off.

The state of Devnet will be reverted to the state of the last accepted block.

E.g. assume there are 3 accepted blocks numbered 1, 2 and 3. Upon receiving a request to abort blocks starting with block 2, the blocks numbered 2 and 3 are aborted and their transactions reverted. The state of network will be as it was in block 1. Once a new block is mined, it will be accepted and it will have number 2.

Aborted blocks can only be queried by block hash. Aborting the blocks in forking origin and already aborted blocks is not supported and results in an error. Aborting of Devnet's genesis block is not supported.

```
POST /abort_blocks
{
    "starting_block_hash": BLOCK_HASH
}
```

```
JSON-RPC
{
    "jsonrpc": "2.0",
    "id": "1",
    "method": "devnet_abortBlocks",
    "params": {
        "starting_block_hash": BLOCK_HASH
    }
}
```

Response:

```
{
    "aborted": [BLOCK_HASH_0, BLOCK_HASH_1, ...]
}
```
