# Blocks

## State commitment

Block states are not committed in a Merke-Patricia trie or a similar tree-like structure. Block roots are therefore set to 0.

## Genesis block

By default, Devnet starts with a genesis block labelled with number zero. In [forking mode](./forking), the genesis block number is equal to the forked block number plus one.

## Limits

To read more about block limits, see [this](./intro#limits).

## Creating blocks on transaction

If you start Devnet with `--block-generation-on transaction`, a new block is generated with each new transaction. This is the default block generation regime. This mode also supports [empty block creation](#request-new-block-creation).

## Creating blocks on demand

If you start Devnet with the `--block-generation-on demand` CLI option, you will enable the possibility to store more than one transaction in the pending block (targetable via block tag `"pending"`).

Once you've added the desired transactions into the pending block, you can [request new block creation](#request-new-block-creation). This will convert the pending block to the latest block (targetable via block tag `"latest"`), giving it a block hash and a block number. All subsequent transactions will be stored in a new pending block.

In case of demanding block creation with no pending transactions, a new empty block will be generated.

The creation of the genesis block is not affected by this feature.

The specifications of a block-creating request can be found [below](#request-new-block-creation).

## Automatic periodic block creation

If started with the `--block-generation-on <INTERVAL>` CLI option, Devnet will behave as in [`demand` mode](#creating-blocks-on-demand), but new blocks will be mined automatically every `<INTERVAL>` seconds. Consider this example of spawning Devnet at moment `t`:

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

## Request new block creation

To request the creation of a new block, `POST` a request with no body to `/create_block` or send:

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

The newly created block will contain all pending transactions, if any, since the last block creation.

### Timestamp manipulation

To affect the timestamp of the newly created block, check out [this page](./starknet-time#set-time)

## Abort blocks

This functionality allows simulating block abortion that can occur on mainnet. It is supported in the `--state-archive-capacity full` mode.

You can abort blocks and revert transactions from the specified block to the currently latest block. Newly created blocks after the abortion will have accepted status and will continue with numbering where the last accepted block left off.

The state of Devnet will be reverted to the state of the last accepted block.

### Example

Assume there are 3 accepted blocks numbered 1, 2 and 3. Upon receiving a request to abort blocks starting with block 2, the blocks numbered 2 and 3 are aborted and their transactions reverted. The state of network will be as it was in block 1. Once a new block is mined, it will be accepted and it will have number 2.

### Limitations

Aborted blocks can only be queried by block hash. Devnet does not support the abortion of:

- blocks in the forking origin (i.e. blocks mined before the forked block)
- already aborted blocks
- Devnet's genesis block

### Websocket subscription notifications

On block abortion, a `starknet_subscriptionReorg` notification will be sent to all websocket subscribers requiring so according to [JSON-RPC websocket API specification](https://github.com/starkware-libs/starknet-specs/blob/v0.8.0-rc1/api/starknet_ws_api.json#L256). The `starting_block` of the orphaned chain is the successor of the new latest block and the `ending_block` of the orphaned chain is the block that was latest before aborting. One reorg notification is sent per subscription, not per websocket, meaning that if a websocket has n subscriptions, it will receive n reorg notifications, each with its own subscription ID.

If a socket has subscribed to transaction status changes of a transaction `tx1` using `starknet_subscribeTransactionStatus` and the block holding `tx1` gets aborted, a `starknet_subscriptionTransactionStatus` notification shall NOT be sent. The socket shall have to rely on handling `starknet_subscriptionReorg`.

### Request and response

To abort, send one of the following:

```
POST /abort_blocks
{
    "starting_block_id": BLOCK_ID
}
```

```
JSON-RPC
{
    "jsonrpc": "2.0",
    "id": "1",
    "method": "devnet_abortBlocks",
    "params": {
        "starting_block_id": BLOCK_ID
    }
}
```

Response:

```
{
    "aborted": [BLOCK_HASH_0, BLOCK_HASH_1, ...]
}
```

Note: When aborting block with tag `pending`, block is mined and aborted as latest.
