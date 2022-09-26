---
sidebar_position: 4
---

# Dumping & Loading

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

## Loading

To load a preserved Devnet instance, the options are:

- Loading on startup:

```
starknet-devnet --load-path <PATH>
```

- Loading on request:

```
curl -X POST http://<HOST>:<PORT>/load -d '{ "path": <PATH> }' -H "Content-Type: application/json"
```

## Enabling dumping and loading with Docker

To enable dumping and loading if running Devnet in a Docker container, you must bind the container path with the path on your host machine.

This example:

- Relies on [Docker bind mount](https://docs.docker.com/storage/bind-mounts/); try [Docker volume](https://docs.docker.com/storage/volumes/) instead.
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
