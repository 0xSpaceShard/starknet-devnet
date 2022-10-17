---
sidebar_position: 12
---
# Contract debugging

If your contract is using `print` in cairo hints (it was compiled with the `--disable-hint-validation` flag), Devnet will output those lines together with its regular server output. Read more about hints [here](https://www.cairo-lang.org/docs/how_cairo_works/hints.html). 

To filter out just your debug lines, redirect stderr to /dev/null when starting Devnet:

```
starknet-devnet 2> /dev/null
```

To disable all the python logging you have to explicitly pass `PYTHONUNBUFFERED=0`:

```
docker run -p 127.0.0.1:5050:5050 -e PYTHONUNBUFFERED=0 shardlabs/starknet-devnet
```
