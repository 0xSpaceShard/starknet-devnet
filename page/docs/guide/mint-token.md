---
sidebar_position: 14
---


# Mint token - Local faucet

Other than using prefunded predeployed accounts, you can also add funds to an account that you deployed yourself.

The ERC20 contract used for minting ETH tokens and charging fees is at: `0x62230ea046a9a5fbc261ac77d03c8d41e5d442db2284587570ab46455fd2488`

## Query fee token address

```
GET /fee_token
```

Response:

```
{
  "symbol":"ETH",
  "address":"0x62230ea046a9a5fbc261ac77d03c8d41e5d442db2284587570ab46455fd2488",
}
```

## Mint with a transaction

By not setting the `lite` parameter or by setting it to `false`, new tokens will be minted in a separate transaction. You will receive the hash of this transaction, as well as the new balance after minting in the response.

`amount` needs to be an integer (or a float whose fractional part is 0, e.g. `1000.0` or `1e21`)

```
POST /mint
{
    "address": "0x6e3205f...",
    "amount": 500000
}
```

Response:

```
{
    "new_balance": 500000,
    "unit": "wei",
    "tx_hash": "0xa24f23..."
}
```

## Mint lite

By setting the `lite` parameter, new tokens will be minted without generating a transaction, thus executing faster.

```
POST /mint
{
    "address": "0x6e3205f...",
    "amount": 500000,
    "lite": true
}
```

Response:

```
{
    "new_balance": 500000,
    "unit": "wei",
    "tx_hash": null
}
```