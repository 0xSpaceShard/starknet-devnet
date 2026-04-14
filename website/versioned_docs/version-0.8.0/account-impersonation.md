# Account impersonation

:::info

This page is about account impersonation. To read about account class selection and deployment, click [here](./predeployed).

:::

## Introduction

Devnet allows you to impersonate an account that exists on the Starknet mainnet or testnet. This is achieved by skipping the validation step of transactions for all or some accounts, on a running Devnet via JSON-RPC.

A transaction sent from an impersonated account will not fail with an invalid signature error, which is what happens in the general case of locally absent accounts. For impersonation to work, Devnet needs to [fork](./forking.md) the network that has the desired account.

:::warning Caveat

- Only `INVOKE` and `DECLARE` transactions are supported. `DEPLOY_ACCOUNT` transaction is not supported, but you can create an `INVOKE` transaction to UDC.
- Due to the validation step being skipped, the overall fee of transactions sent with an impersonated account will be lower than regular transactions.
- Trying to send a transaction with an account that **does not** even exist in the origin network returns an error:
  - `ContractNotFound` if, during transaction preparation, you do not specify a nonce value, leading to the implicit querying of Devnet for the nonce.
  - `InsufficientAccountBalance` or similar if the nonce is supplied in the transaction; this happens because the token balance of a non-existent contract is 0 indeed insufficient.

:::

## Tips

- The impersonated account may have had all or a part of its funds used up on the origin network. You may need to give it more funds via [minting](./balance.md).
- If you're defining a new account in your Starknet client application (starknet.js, starknet.rs, starkli...), you may need to specify a private key for it. Since the signature validation is skipped, you may provide a dummy key.

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

## Preventing impersonation

If you want to learn about completely preventing impersonation from being activated on your Devnet, click [here](./restrictive.md).
