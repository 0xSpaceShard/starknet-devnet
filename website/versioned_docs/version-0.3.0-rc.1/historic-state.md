# Historic state support

With state archive capacity set to `full`, Devnet will store full state history, enabling its querying by block hash or number. The default mode is `none`, where no old states are stored and only the latest is available for querying.

```
$ starknet-devnet --state-archive-capacity <CAPACITY>
```

All RPC endpoints that support querying the state at an old (non-latest) block only work with state archive capacity set to `full`.
