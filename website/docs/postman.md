# L1-L2 interaction via Postman

Postman is a Starknet utility that allows testing L1-L2 interaction. It is **unrelated** to the [Postman API platform](https://www.postman.com/). To use it, ensure you have:

- an L1 node (possibilities listed [below](#l1-network))
- a Devnet instance (acting as L2 node)
- a [loaded](#load) messaging contract
  - this is an L1 contract for exchanging messages between L1 and L2
  - you can deploy a new instance or specify the address of an old one
- an L1 contract that can interact with the messaging contract
- an L2 contract that can interact with the messaging contract

There are two internal message queues: one for L1->L2 messages, another for L2->L1 messages. When there are messages in a queue, you will need to [flush](#flush) to transmit the messages to their destinations.

You can use [**`starknet-devnet-js`**](https://github.com/0xSpaceShard/starknet-devnet-js) to assist you in the above listed actions. [**This example**](https://github.com/0xSpaceShard/starknet-devnet-js/blob/master/test/l1-l2-postman.test.ts), especially the `it("should exchange messages between L1 and L2")` test case should be of most help. The required contracts are configured and deployed in the `before` block. Alternatively, you can directly send requests to the endpoints specified below.

## Load

```json
// JSON-RPC
{
  "jsonrpc": "2.0",
  "id": "1",
  "method": "devnet_postmanLoad",
  "params": {
    "network_url": "http://localhost:8545",
    "messaging_contract_address": "0x123...def", // optional
    "deployer_account_private_key": "0xe2ac...583f" // optional
  }
}
```

Loads an L1 `MockStarknetMessaging` contract instance, potentially deploying a new one, which is used for message exchange between L1 and L2.

### L1 network

The `network_url` parameter refers to the URL of the JSON-RPC API endpoint of the L1 node you've run locally, or which is publicly accessible. Possibilities include, but are not limited to:

- [**Anvil**](https://github.com/foundry-rs/foundry/tree/master#anvil)
- [**Sepolia testnet**](https://sepolia.etherscan.io/)
- [**Ganache**](https://www.npmjs.com/package/ganache)
- [**Geth**](https://github.com/ethereum/go-ethereum#docker-quick-start)
- [**Hardhat node**](https://hardhat.org/hardhat-network/#running-stand-alone-in-order-to-support-wallets-and-other-software)

### Messaging contract deployment

Here's how the rest of the parameters should be used, depending on your L1 network:

- If your L1 network already has a messaging contract deployed that you wish to use, populate `messaging_contract_address` with its address.
  - The provided address shall be checked by asserting that there indeed is contract code deployed at that address, without any ABI assertions.
- If your L1 network does not have such a contract, or you simplify wish to deploy a new instance, leave out the `messaging_contract_address` property.
  - If your L1 network is a local testnet (e.g. Anvil) with the default mnemonic seed and a default set of predeployed accounts, you don't have to specify anything else—a new messaging contract shall be deployed using a predeployed account.
  - Otherwise (e.g. on the Sepolia testnet or an Anvil with a custom mnemonic seed) you are expected to populate `deployer_account_private_key` with the private key of a funded account. This property is not applicable if `messaging_contract_address` is specified.

:::note L1-L2 with dockerized Devnet

L1-L2 communication requires extra attention if Devnet is [run in a Docker container](./running/docker.md). The `network_url` argument must be on the same network as Devnet. E.g. if your L1 instance is run locally (i.e. using the host machine's network), then:

- on Linux, it is enough to run the Devnet Docker container with `--network host`
- on Mac and Windows, replace any `http://localhost` or `http://127.0.0.1` occurrence in the value of `network_url` with `http://host.docker.internal`.

:::

:::info Dumping and Loading

Loading a messaging contract is a dumpable event, meaning that, if you've enabled dumping, a messaging-contract-loading event will be dumped. Keep in mind that, if you rely on Devnet deploying a new contract, i.e. if you don't specify a contract address of an already deployed messaging contract, a new contract will be deployed at a new address on each loading of the dump. Read more about dumping [here](./dump-load-restart#dumping).

:::

## Flush

```
JSON-RPC
{
    "jsonrpc": "2.0",
    "id": "1",
    "method": "devnet_postmanFlush"
}
```

Goes through the newly enqueued messages since the last flush, consuming and sending them from L1 to L2 and from L2 to L1. Use it for end-to-end testing. Requires no body. Optionally, set the `dry_run` boolean flag to just see the result of flushing, without actually triggering it:

```
JSON-RPC
{
    "jsonrpc": "2.0",
    "id": "1",
    "method": "devnet_postmanFlush",
    "params": { "dry_run": true }
}
```

:::note

A running L1 node is required if `dry_run` is not set.

:::

:::info Dumping and Loading

Flushing is not dumpable, meaning that, if you've enabled dumping, a flushing event will not itself be re-executed on loading. This is because it produces L2 messaging events that are themselves dumped. No L1-side actions are dumped, you need to take care of those yourself. Read more about dumping [here](./dump-load-restart#dumping).

:::

## Disclaimer

This method of L1-L2 communication testing differs from how Starknet mainnet and testnets work. Taking [**L1L2Example.sol**](https://github.com/MikeSpa/starknet-test/blob/6a68d033cd7ddb5df937154f860f1c06174e6860/L1L2Example.sol#L46) (originally from Starknet documentation, no longer available there):

```solidity
constructor(IStarknetCore starknetCore_) public {
    starknetCore = starknetCore_;
}
```

The constructor takes an `IStarknetCore` contract as argument, however for Devnet's L1-L2 communication testing, this has to be replaced with the logic in [**MockStarknetMessaging.sol**](https://github.com/starkware-libs/cairo-lang/blob/master/src/starkware/starknet/testing/MockStarknetMessaging.sol):

```solidity
constructor(MockStarknetMessaging mockStarknetMessaging_) public {
    starknetCore = mockStarknetMessaging_;
}
```

## Mock transactions

### L1->L2

:::note

A running L1 node is **not** required for this operation.

The L2 target entrypoint must be an `l1_handler`.

:::

Sends a mock transactions to L2, as if coming from L1, without the need for running L1. The target L2 contract's address must be provided to `l2_contract_address` and the `entry_point_selector` must refer to a public method of the target contract. The method must be annotated with `l1_handler`, otherwise an `ENTRYPOINT_NOT_FOUND` error may be returned. The `l1_transaction_hash` property is optional and, if provided, enables future `starknet_getMessagesStatus` requests with that hash value provided.

In regular (non-mocking) L1-L2 interaction, `nonce` is determined by the L1 Starknet contract. In this mock case, it is up to the developer to set it.

```
JSON-RPC
{
    "jsonrpc": "2.0",
    "id": "1",
    "method": "devnet_postmanSendMessageToL2",
    "params": {
      "l2_contract_address": "0x00285ddb7e5c777b310d806b9b2a0f7c7ba0a41f12b420219209d97a3b7f25b2",
      "entry_point_selector": "0xC73F681176FC7B3F9693986FD7B14581E8D540519E27400E88B8713932BE01",
      "l1_contract_address": "0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512",
      "payload": [ "0x1", "0x2" ],
      "paid_fee_on_l1": "0x123456abcdef",
      "nonce": "0x0",
      "l1_transaction_hash": "0x000abc123", // optional
  }
}
```

Result:

```js
{ "transaction_hash": "0x0548c761a9fd5512782998b2da6f44c42bf78fb88c3794eea330a91c9abb10bb" }
```

### L2->L1

Sends a mock transaction from L2 to L1. The deployed L2 contract address `from_address` and `to_address` must be valid.

It is a mock message, but only in the sense that you are mocking an L2 contract's action, which would normally be triggered by invoking the contract via a transaction. So keep in mind the following:

:::note

A running L1 node is required for this operation.

:::

```
JSON-RPC
{
    "jsonrpc": "2.0",
    "id": "1",
    "method": "devnet_postmanConsumeMessageFromL2",
    "params": {
      "from_address": "0x00285ddb7e5c777b310d806b9b2a0f7c7ba0a41f12b420219209d97a3b7f25b2",
      "to_address": "0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512",
      "payload": ["0x0", "0x1", "0x3e8"],
  }
}
```

Result:

```js
{"message_hash": "0xae14f241131b524ac8d043d9cb4934253ac5c5589afef19f0d761816a9c7e26d"}
```
