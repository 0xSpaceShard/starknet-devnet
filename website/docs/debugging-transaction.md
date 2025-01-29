# Debugging transactions

:::

## Introduction

Devnet allows you to debug a transaction. This feature is provided in collaboration with [Walnut](https://walnut.dev). To use the debugging feature you have to start devnet with `--state-archive-capacity full`, provide `ngrok` authentication token and `Walnut` API key (Either via environment variables `NGROK_AUTHENTICATION_TOKEN` , `WALNUT_API_KEY` or command-line parameters `--ngrok <NGROK_AUTHTOKEN>`, `--walnut <WALNUT_API_KEY>`).

:::warning Caveat

- Only `INVOKE` transactions are supported.
- Debugging **pending** transactions is not supported.

:::

## API

Transaction debugging follows JSON-RPC method specification.
</br>
All of the methods require smart contract code to be uploaded. It can be done in the following ways:

1. **Specifying the Path to the Cairo Workspace**: Provide the path to the directory containing your Cairo smart contract files.
2. **Sending a JSON Object**: Provide a JSON object where:
   - **Keys**: The name of the file (including the extension).
   - **Values**: The content of the file.

### devnet_debugTransaction

Uploads the smart contract to Walnut and returns a URL for the debug session in the Walnut system.

### Request:
```js
{
    "jsonrpc": "2.0",
    "method": "devnet_debugTransaction",
    "id": 0,
    "params": {
        "contract_source": {
            "path": "/user/projects/cairo-contracts/hello_world"
        },
        "target": {
            "transaction_hash": "0x000131"
        }
    }
}
```

```js
{
    "jsonrpc": "2.0",
    "method": "devnet_debugTransaction",
    "id": 0,
    "params": {
        "contract_source": {
            "lib.cairo": "<cairo contract content>",
            "mod.cairo": "<cairo contract content>",
        },
        "target": {
            "transaction_hash": "0x000131"
        }
    }
}
```

### Response:

```js
{
    "jsonrpc":"2.0",
    "id":0,
    "result":
    "https://app.walnut.dev/transactions?rpcUrl=https%3A%2F%2Ffaf1-2a01-5a8-30a-660-f856-b51e-9a1e-baee.ngrok-free.app&txHash=0x0168e4e2be84ea914de913b2d21151d64489126ea9eaa6370bf7fdd73b26a638"
}
```

### devnet_walnutVerifyContract

Uploads the smart contract code and returns the response from Walnut, it contains URL for tracking the status of verification.
Specifying the `sierra_artifact_source` parameter can be done in the following ways:
1. **Specifying the Path to the sierra file**: Provide the path to the file with the smart contract sierra representation. It is located mostly in `./target/dev/` directory (relative to the directory where `scarb build` is executed) and has extension `.contract_class.json`.
2. **Sending a JSON Object**: Provide a JSON object with the contents of the sierra file.

### Request

```js
{
    "jsonrpc": "2.0",
    "method": "devnet_debugTransaction",
    "id": 0,
    "params": {
        "contract_source": {
            "path": "/user/projects/cairo-contracts/hello_world"
        },
        "sierra_artifact_source": {
            "path": "/cairo-contracts/account/target/dev/account_Account.contract_class.json"
        }
    }
}
```

```js
{
    "jsonrpc": "2.0",
    "method": "devnet_debugTransaction",
    "id": 0,
    "params": {
        "contract_source": {
            "lib.cairo": "<cairo contract content>",
            "mod.cairo": "<cairo contract content>",
        },
        "sierra_artifact_source": {
            "sierra_program": [],
            ...
        }
    }
}
```

### Response
```js
{
    "jsonrpc": "2.0",
    "id": 0,
    "result": "\"Contract verification has started. You can check the verification status at the following link: https://app.walnut.dev/verification/status/5050666b-d4a2-4c2e-afc4-1c4a1cb3eb8b\""
}
```
