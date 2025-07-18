# Lite mode

To run Devnet in a minimal lite mode, provide the flag:

```
$ starknet-devnet --lite-mode
```

Steps skipped in lite mode:

- calculating block hash

This is useful if your use-case doesn't need the functionalities above.

The extent of what is affected by lite mode may be expanded in the future.
