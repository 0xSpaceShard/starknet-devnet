# Account impersonation

:::info

This page is about account impersonation. To read about account class selection and deployment, click [here](./predeployed).

:::

Devnet allows you to use impersonated account from mainnet/testnet. This means that a transaction sent from an impersonated account will not fail with an invalid signature error. In the general case, a transaction sent with an account that is not in the local state fails with the aforementioned error. For impersonation to work, Devnet needs to be run in [forking mode](./forking.md).

:::warning Caveat

- Only `INVOKE` and `DECLARE` transactions are supported. `DEPLOY_ACCOUNT` transaction is not supported, but you can create an `INVOKE` transaction to UDC.
- Overall fee, for transactions sent with an impersonated account, will be lower compared to normal transactions. The reason is that validation part is skipped.
- The most common way of sending a transaction is via starknet-rs/starknet.js or starkli. Trying to send with an account that **does not** exist even in the origin network will return an error:
  - In transaction construction, if account nonce is not hardcoded, Devnet is queried and returns `ContractNotFound`.
  - Otherwise the nonce fetching part is skipped and `InsufficientAccountBalance` is returned.

:::

## API

Account impersonation follows JSON-RPC method specification. Each method returns an empty response:

### devnet_impersonateAccount

Impersonates a specific account address nonexistent in the local state.

```js
{
    "jsonrpc": "2.0",
    "id": "1",
    "method": "devnet_impersonateAccount",
    "params": {
        "account_address": "0x49D36570D4E46F48E99674BD3FCC84644DDD6B96F7C741B1562B82F9E004DC7"
    }
}
```

### devnet_stopImpersonateAccount

Stops the impersonation of an account previously marked for impersonation.

```js
{
    "jsonrpc": "2.0",
    "id": "1",
    "method": "devnet_stopImpersonateAccount",
    "params": {
        "account_address": "0x49D36570D4E46F48E99674BD3FCC84644DDD6B96F7C741B1562B82F9E004DC7"
    }
}
```

### devnet_autoImpersonate

Enables automatic account impersonation. Every account that does not exist in the local state will be impersonated.

```js
{
    "jsonrpc": "2.0",
    "id": "1",
    "method": "devnet_autoImpersonate",
    "params": {}
}
```

### devnet_stopAutoImpersonate

Stops the effect of [automatic impersonation](#devnet_autoimpersonate).

```js
{
    "jsonrpc": "2.0",
    "id": "1",
    "method": "devnet_stopAutoImpersonate",
    "params": {}
}
```
