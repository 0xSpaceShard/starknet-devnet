---
sidebar_position: 6
---

# L1-L2 Postman integration

Postman is a StarkNet utility that allows testing L1 <-> L2 interaction. To utilize this, you can use [**`starknet-hardhat-plugin`**](https://github.com/Shard-Labs/starknet-hardhat-plugin), as witnessed in [**this example**](https://github.com/Shard-Labs/starknet-hardhat-example/blob/master/test/postman.test.ts). Or you can directly interact with the two Postman-specific endpoints:

### Postman - Load

```
POST /postman/load_l1_messaging_contract
{
  "networkUrl": "http://localhost:8545",
  "address": "0x123...def"
}
```

Loads a `StarknetMockMessaging` contract. The `address` parameter is optional; if provided, the `StarknetMockMessaging` contract will be fetched from that address, otherwise a new one will be deployed.

`networkUrl` is the URL of the JSON-RPC API of the L1 node you've run locally or that already exists; possibilities include, and are not limited to:

- [**Goerli testnet**](https://goerli.net/)
- [**Ganache**](https://www.npmjs.com/package/ganache)
- [**Geth**](https://github.com/ethereum/go-ethereum#docker-quick-start)
- [**Hardhat node**](https://hardhat.org/hardhat-network/#running-stand-alone-in-order-to-support-wallets-and-other-software).

### Postman - Flush

```
POST /postman/flush
```
Goes through the newly enqueued messages, sending them from L1 to L2 and from L2 to L1. Requires no body.

### Postman - disclaimer

This method of L1 <-> L2 communication testing differs from StarkNet Alpha networks. Taking the [**L1 L2 Example .sol**](https://www.cairo-lang.org/docs/_static/L1L2Example.sol) contract in the [**starknet documentation**](https://www.cairo-lang.org/docs/hello_starknet/l1l2.html) :

```
constructor(IStarknetCore starknetCore_) public {
    starknetCore = starknetCore_;
}
```

The constructor takes an `IStarknetCore` contract as argument, however for Devnet L1 <-> L2 communication testing, this will have to be replaced with the [**MockStarknetMessaging.sol**](https://github.com/starkware-libs/cairo-lang/blob/master/src/starkware/starknet/testing/MockStarknetMessaging.sol) contract :

```
constructor(MockStarknetMessaging mockStarknetMessaging_) public {
    starknetCore = mockStarknetMessaging_;
}
```

### Postman - l1 to l2 mock endpoint

Sending mock transactions from L1 to L2 without the need for running L1. Deployed L2 contract address `l2_contract_address` and `entry_point_selector` must be valid otherwise new block will not be created.

Normally `nonce` is calculated by l1 StarknetContract and it's used in L1 and L2. In this case, we need to provide it manually.


```
POST /postman/send_message_to_l2
```

Request:
```
{
    "l2_contract_address":"0x00285ddb7e5c777b310d806b9b2a0f7c7ba0a41f12b420219209d97a3b7f25b2",
    "entry_point_selector":"0xC73F681176FC7B3F9693986FD7B14581E8D540519E27400E88B8713932BE01",
    "l1_contract_address":"0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512",
    "payload":[
      "0x1",
      "0x2"
    ],
    "nonce":"0x0"
}
```

Response: 
```
{'transaction_hash': "0x0548c761a9fd5512782998b2da6f44c42bf78fb88c3794eea330a91c9abb10bb"}
```

### Postman - l2 to l1 mock endpoint

Sending mock transactions from L2 to L1.
Deployed L2 contract address `l2_contract_address` and `l1_contract_address` must be valid.

```
POST /postman/consume_message_from_l2
```

Request:
```
{
    "l2_contract_address": "0x00285ddb7e5c777b310d806b9b2a0f7c7ba0a41f12b420219209d97a3b7f25b2",
    "l1_contract_address": "0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512",
    "payload": ["0x0", "0x1", "0x3e8"],
}
```

Response: 
```
{'message_hash': '0xae14f241131b524ac8d043d9cb4934253ac5c5589afef19f0d761816a9c7e26d'}
```

## Dumping

To preserve your Devnet instance for future use, there are several options:

- Dumping on exit (handles Ctrl+C, i.e. SIGINT, doesn't handle SIGKILL):

```
starknet-devnet --dump-on exit --dump-path <PATH>
```

- Dumping after each transaction (done in background, doesn't block):

```
starknet-devnet --dump-on transaction --dump-path <PATH>
```

- Dumping on request (replace `<HOST>`, `<PORT>` and `<PATH>` with your own):

```
curl -X POST http://<HOST>:<PORT>/dump -d '{ "path": <PATH> }' -H "Content-Type: application/json"
```

### Loading

To load a preserved Devnet instance, the options are:

- Loading on startup:

```
starknet-devnet --load-path <PATH>
```

- Loading on request:

```
curl -X POST http://<HOST>:<PORT>/load -d '{ "path": <PATH> }' -H "Content-Type: application/json"
```

### Enabling dumping and loading with Docker

To enable dumping and loading if running Devnet in a Docker container, you must bind the container path with the path on your host machine.

This example:

- Relies on [**Docker bind mount**](https://docs.docker.com/storage/bind-mounts/); try [**Docker volume**](https://docs.docker.com/storage/volumes/) instead.
- Assumes that `/actual/dumpdir` exists. If unsure, use absolute paths.
- Assumes you are listening on `127.0.0.1:5050`.

If there is `dump.pkl` inside `/actual/dumpdir`, you can load it with:

```
docker run \
  -p 127.0.0.1:5050:5050 \
  --mount type=bind,source=/actual/dumpdir,target=/dumpdir \
  shardlabs/starknet-devnet \
  --load-path /dumpdir/dump.pkl
```

To dump to `/actual/dumpdir/dump.pkl` on Devnet shutdown, run:

```
docker run \
  -p 127.0.0.1:5050:5050 \
  --mount type=bind,source=/actual/dumpdir,target=/dumpdir \
  shardlabs/starknet-devnet \
  --dump-on exit --dump-path /dumpdir/dump.pkl
```
