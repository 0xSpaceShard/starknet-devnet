# Cairo Contracts

This folder contains a Scarb package to compile and deploy Cairo 1
contracts on Devnet for development purposes.

## Work with Scarb

Start by installing Scarb (with `asdf` **highly** recommended) [from the tutorial](https://docs.swmansion.com/scarb/).
Ensure you've at least version `2.3.1` installed.

### Build

To build contracts, use:
```bash
scarb build
```

The contracts artifacts are generated into `target/dev` folder.
Two files can be found there:
* The Sierra class file: `package_contract.contract_class.json`
* The compiled CASM file: `package_contract.compiled_contract_class.json`

### Interact with Devnet

To interact with Devnet, [Starkli](https://book.starkli.rs/) is the easiest CLI tool to use.
To work with Starkli, you need two files:
* The keystore file with the private key being encrypted there. This file can also be replaced by the private
  key in plain text, which is totally fine for testing.
* The account file with the account definition and address.

To ease the development, start the Devnet with a known seed (`--seed 42`) to use the pre-built account file.

```bash
# On a first terminal, run the Devnet with a known seed `cargo run -- --seed 42`.

# Export variables to have starkli pre-configured.
source ./env

# Declare
starkli declare target/dev/cairo_l1_l2.contract_class.json

# Deploy (adjust the class hash if needed).
starkli deploy 0x0211fd0483be230ba40d43f51bd18ae239b913f529f95ce10253e514175efb3e --salt 123

# Interact with the contract
starkli invoke 0x03c80468c8fe2fd36fadf1b484136b4cd8a372f789e8aebcc6671e00101290a4 increase_balance 0x1 0xff
starkli call 0x03c80468c8fe2fd36fadf1b484136b4cd8a372f789e8aebcc6671e00101290a4 get_balance 0x1

# To send messages to L1, you can use:
starkli invoke 0x03c80468c8fe2fd36fadf1b484136b4cd8a372f789e8aebcc6671e00101290a4 withdraw 0x1 1 0x112233

# Then you can check the message ready to be sent with the `postman/flush` endpoint, with a dry run without actually
# running a L1 node:
curl -H 'Content-Type: application/json' -d '{"dryRun": true}' http://127.0.0.1:5050/postman/flush

{
    "messagesToL1":[
        {
            "l2_contract_address":"0x34ba56f92265f0868c57d3fe72ecab144fc96f97954bbbc4252cef8e8a979ba",
            "l1_contract_address":"0x112233",
            "payload":["0x0","0x1","0x1"]
        }
    ],
    "messagesToL2":[],
    "l1Provider":"dry run"
}

# If you want to simulate a message arriving from the L1 without actually running a L1 node, you can trigger
# the l1_handler `deposit` function by using the postman `send_message_to_l2` endpoint:
curl -H 'Content-Type: application/json' \
     -d '{"paidFeeOnL1": "0x123", "l2ContractAddress": "0x03c80468c8fe2fd36fadf1b484136b4cd8a372f789e8aebcc6671e00101290a4", "l1ContractAddress": "0x112233", "entryPointSelector": "0x00c73f681176fc7b3f9693986fd7b14581e8d540519e27400e88b8713932be01", "payload": ["0x1", "0x2"], "nonce": "0x1"}' \
     http://127.0.0.1:5050/postman/send_message_to_l2

{
    "transactionHash":"0x69a3fff03cee2d68013b13679c65073eb673f357fcdeec7f26cabf9893720c6"
}

# You can then check the event being emitted during this transaction to check the deposit (or you can also call
# the contract again to check the balance):
starkli receipt 0x69a3fff03cee2d68013b13679c65073eb673f357fcdeec7f26cabf9893720c6

{
  "type": "L1_HANDLER",
  "transaction_hash": "0x69a3fff03cee2d68013b13679c65073eb673f357fcdeec7f26cabf9893720c6",
  "actual_fee": "0x0",
  "finality_status": "ACCEPTED_ON_L2",
  "block_hash": "0x51d7ee9fa3a6226d47860eea28dc0b38eeccd7b6fac1b9f39c64c3ac772cc02",
  "block_number": 2,
  "messages_sent": [],
  "events": [
    {
      "from_address": "0x315f3c38678ad3b4f8852bf6a8e9d24f3eea2421a76510f3d2aa740bacb0eef",
      "keys": [
        "0x2fcde209a303a2fc48dcf9c7a3b76f69f9b032505d6aa5312a6835bc9f40c88",
        "0x1",
        "0x2"
      ],
      "data": []
    }
  ],
  "execution_status": "SUCCEEDED"
}

```
