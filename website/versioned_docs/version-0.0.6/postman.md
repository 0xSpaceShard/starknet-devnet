# L1-L2 interaction via Postman

Postman is a Starknet utility that allows testing L1-L2 interaction. Ensure you have an L1 node and a Devnet (L2 node) running, [load](#load) a messaging contract, and [flush](#flush) the queue when needed. You can use [**`starknet-hardhat-plugin`**](https://github.com/0xSpaceShard/starknet-hardhat-plugin) to perform these actions, as witnessed in [**this example**](https://github.com/0xSpaceShard/starknet-hardhat-example/blob/master/test/l1-l2-postman.test.ts), or directly send requests to the endpoints specified below.

## Load

```
POST /postman/load_l1_messaging_contract
```

```js
{
  "networkUrl": "http://localhost:8545",
  "address": "0x123...def"
}
```

Loads a `MockStarknetMessaging` contract. The `address` parameter is optional; if provided, the `MockStarknetMessaging` contract will be fetched from that address, otherwise a new one will be deployed.

`networkUrl` is the URL of the JSON-RPC API of the L1 node you've run locally or that already exists; possibilities include, and are not limited to:

- [**Anvil**](https://github.com/foundry-rs/foundry/tree/master/crates/anvil)
- [**Sepolia testnet**](https://sepolia.etherscan.io/)
- [**Ganache**](https://www.npmjs.com/package/ganache)
- [**Geth**](https://github.com/ethereum/go-ethereum#docker-quick-start)
- [**Hardhat node**](https://hardhat.org/hardhat-network/#running-stand-alone-in-order-to-support-wallets-and-other-software)

## Flush

```
POST /postman/flush
```

Goes through the newly enqueued messages, sending them from L1 to L2 and from L2 to L1. Requires no body. Optionally, set the `dry_run` specifier to just see the result of flushing, without actually triggering it:

```
POST /postman/flush
```

```js
{ "dry_run": true }
```

A running L1 node is required if `dry_run` is not set.

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

Sending mock transactions from L1 to L2 without the need for running L1. Deployed L2 contract address `l2_contract_address` and `entry_point_selector` must be valid otherwise new block will not be created.

Normally `nonce` is calculated by L1 StarknetContract and it's used in L1 and L2. In this case, we need to provide it manually.

A running L1 node is **not** required for this operation.

```
POST /postman/send_message_to_l2
```

Request:

```js
{
    "l2_contract_address": "0x00285ddb7e5c777b310d806b9b2a0f7c7ba0a41f12b420219209d97a3b7f25b2",
    "entry_point_selector": "0xC73F681176FC7B3F9693986FD7B14581E8D540519E27400E88B8713932BE01",
    "l1_contract_address": "0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512",
    "payload": [
      "0x1",
      "0x2"
    ],
    "paid_fee_on_l1": "0x123456abcdef"
    "nonce":"0x0"
}
```

Response:

```js
{ "transaction_hash": "0x0548c761a9fd5512782998b2da6f44c42bf78fb88c3794eea330a91c9abb10bb" }
```

### L2->L1

Sending mock transactions from L2 to L1.
Deployed L2 contract address `l2_contract_address` and `l1_contract_address` must be valid.

A running L1 node is required for this operation.

```
POST /postman/consume_message_from_l2
```

Request:

```js
{
    "l2_contract_address": "0x00285ddb7e5c777b310d806b9b2a0f7c7ba0a41f12b420219209d97a3b7f25b2",
    "l1_contract_address": "0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512",
    "payload": ["0x0", "0x1", "0x3e8"],
}
```

Response:

```js
{"message_hash": "0xae14f241131b524ac8d043d9cb4934253ac5c5589afef19f0d761816a9c7e26d"}
```
