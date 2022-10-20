---
sidebar_position: 6
---

# L1-L2 Postman integration

Postman is a Starknet utility that allows testing L1 <-> L2 interaction. To utilize this, you can use [**`starknet-hardhat-plugin`**](https://github.com/Shard-Labs/starknet-hardhat-plugin), as witnessed in [**this example**](https://github.com/Shard-Labs/starknet-hardhat-example/blob/master/test/postman.test.ts). Or you can directly interact with the two Postman-specific endpoints:

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

This method of L1 <-> L2 communication testing differs from Starknet Alpha networks. Taking the [**L1 L2 Example .sol**](https://www.cairo-lang.org/docs/_static/L1L2Example.sol) contract in the [**starknet documentation**](https://www.cairo-lang.org/docs/hello_starknet/l1l2.html) :

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
