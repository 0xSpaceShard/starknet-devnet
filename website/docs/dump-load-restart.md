# Dump, load, restart

## Dumping

To preserve your Devnet instance for future use, these are the options:

- Dumping on exit (handles Ctrl+C, i.e. SIGINT; doesn't handle SIGKILL):

```
$ starknet-devnet --dump-on exit --dump-path <PATH>
```

- Dumping after each block:

```
$ starknet-devnet --dump-on block --dump-path <PATH>
```

- Dumping on request requires providing --dump-on mode on the startup. Example usage in `exit` mode (replace `<HOST>`, `<PORT>` and `<PATH>` with your own):

```
$ starknet-devnet --dump-on exit --dump-path <PATH>
$ curl -X POST http://<HOST>:<PORT>/dump -d '{ "path": <PATH> }' -H "Content-Type: application/json"
```

## Loading

To load a preserved Devnet instance, the options are:

- Loading on startup (note the argument name is not `--load-path` as it was in Devnet-py):

```
$ starknet-devnet --dump-path <PATH>
```

- Loading on request:

```
curl -X POST http://<HOST>:<PORT>/load -d '{ "path": <PATH> }' -H "Content-Type: application/json"
```

Currently, dumping produces a list of received transactions that is stored on disk. Conversely, loading is implemented as the re-execution of transactions from a dump. This means that timestamps of `StarknetBlock` will be different on each load.

### Loading disclaimer

Dumping and loading are not guaranteed to work across versions. I.e. if you dumped one version of Devnet, do not expect it to be loadable with a different version.

If you dumped a Devnet utilizing one class for account predeployment (e.g. `--account-class cairo0`), you should use the same option when loading. The same applies for dumping a Devnet in `--block-generation-on demand` mode.

## Restarting

Devnet can be restarted by making a `POST /restart` request (no body required). All of the deployed contracts (including predeployed), blocks and storage updates will be restarted to the original state, without the transactions and requests that may have been loaded from a dump file on startup.

If you're using [**the Hardhat plugin**](https://github.com/0xSpaceShard/starknet-hardhat-plugin#restart), restart with `starknet.devnet.restart()`.
