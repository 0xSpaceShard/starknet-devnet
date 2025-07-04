# Starknet time

Block and state timestamp can be manipulated by setting the exact time or setting the time offset. By default, timestamp methods `/set_time`, `/increase_time` and `JSON-RPC` methods `devnet_setTime`, `devnet_increaseTime` generate a new block. This can be changed for `/set_time` (`devnet_setTime`) by setting the optional parameter `generate_block` to `false`. This skips immediate new block generation, but will use the specified timestamp whenever the next block is supposed to be generated.

All values should be set in [Unix time seconds](https://en.wikipedia.org/wiki/Unix_time). After [startup](#start-time), the time progresses naturally.

## Set time

The following sets the exact time and generates a new block:

```
POST /set_time
{
    "time": TIME_IN_SECONDS
}
```

```
JSON-RPC
{
    "jsonrpc": "2.0",
    "id": "1",
    "method": "devnet_setTime",
    "params": {
        "time": TIME_IN_SECONDS
    }
}
```

The following doesn't generate a new block, but sets the exact time for the next generated block:

```
POST /set_time
{
    "time": TIME_IN_SECONDS,
    "generate_block": false
}
```

```
JSON-RPC
{
    "jsonrpc": "2.0",
    "id": "1",
    "method": "devnet_setTime",
    "params": {
        "time": TIME_IN_SECONDS,
        "generate_block": false
    }
}
```

Warning: block time can be set in the past which might lead to unexpected behavior!

## Increase time

Increases the block timestamp by the provided amount and generates a new block. All subsequent blocks will keep this increment.

```
POST /increase_time
{
    "time": TIME_IN_SECONDS
}
```

```
JSON-RPC
{
    "jsonrpc": "2.0",
    "id": "1",
    "method": "devnet_increaseTime",
    "params": {
        "time": TIME_IN_SECONDS
    }
}
```

:::note Increment example

Imagine a block is generated with timestamp `T1`, some time passes (let's call this interval `T_passed`), and you call increase_time with `T_inc` as the argument, which immediately mines a new block at `T2`. `T2` should equal `T1 + T_passed + T_inc`.

:::

## Start time

Devnet's starting timestamp can be defined via CLI by providing a positive value of [Unix time seconds](https://en.wikipedia.org/wiki/Unix_time) to `--start-time`:

```
$ starknet-devnet --start-time <SECONDS>
```

If provided, this timestamp shall be used in mining the genesis block. The default value is the current Unix time.
