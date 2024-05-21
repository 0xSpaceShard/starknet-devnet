# Devnet contracts for development

This folder contains Cairo and Solidity contracts for Devnet developement.
If you wish to check specifically one of the two chains README, please refer to the corresponding README:
1. `solidity` folder for Ethereum related contracts with the [README](./solidity/README.md).
2. `cairo` folder for Starknet related contracts, and example of how to work with starknet without running an L1 node in the [README](./cairo/README.md).

## E2E testing with Anvil and Devnet

### Setup of the nodes
You will need two terminals to run each node:

First, please ensure that you have [anvil](https://book.getfoundry.sh/getting-started/installation) installed (or you can do the same with HardHat, but the commands here are done with anvil).
```bash
anvil
```

For Starknet, ensure you have Devnet compiled and running with the following params:
```bash
# First, ensure you have compiled the artifacts required for abigen:
cd contracts && bash generate_artifacts.sh

# Then run Devnet with known seed.
cargo run -- --seed 42
```

Now both nodes are running, Devnet for Starknet and Anvil for Ethereum.

Then, open a third terminal **in the same directory of this README**, from which we will operate on the running nodes:
```bash
# This .env file combines variables for both chain.
source ./.env

# Compile cairo contracts.
scarb --manifest-path ./cairo/Scarb.toml build

# Compile solidity contracts.
forge install --root ./solidity
forge build --root ./solidity
```

### Ethereum setup
1. Use Devnet postman endpoint to load the `MockStarknetMessaging` contract:
```bash
curl -H 'Content-Type: application/json' \
     -d '{"network_url": "http://127.0.0.1:8545"}' \
     http://127.0.0.1:5050/postman/load_l1_messaging_contract
```
```json
{
    "messaging_contract_address":"0x5fbdb2315678afecb367f032d93f642f64180aa3"
}
```

2. Deploy the `L1L2.sol` contract in order to receive/send messages from/to L2.
```bash
pushd ./solidity
forge script ./script/L1L2.s.sol:Deploy --broadcast --rpc-url $ETH_RPC_URL
popd
```
```
✅  [Success]Hash: 0x942cfaadc557f360b91e2bfe98e8246d87b8efb4bfe6c1803162cd4aa7a71e1d
Contract Address: 0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512
Block: 2
Paid: 0.0013459867197597 ETH (346581 gas * 3.8836137 gwei)
```

3. Check balance is 0 for user `0x1`:
```bash
cast call 0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512 "get_balance(uint256)(uint256)" 0x1
```
```bash
0
```

### Starknet contracts and send message to L1
1. On Devnet, we will declare and deploy the `cairo_l1_l2` contract to send-receive messages on the Starknet side:
```bash
# Declare.
starkli declare ./cairo/target/dev/cairo_l1_l2.contract_class.json
CLASS_HASH= # displayed after previous command

# Deploy (adjust the class hash if needed).
starkli deploy "$CLASS_HASH" --salt 123
CONTRACT_L2= # displayed after previous command

# Add some balance (255) to the user 1 on L2 and check it.
starkli invoke "$CONTRACT_L2" increase_balance 0x1 0xff
starkli call "$CONTRACT_L2" get_balance 0x1

# Issue a withdraw to send message to L1 with amount 1 for user 1.
starkli invoke "$CONTRACT_L2" withdraw 0x1 0x1 0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512

# Here, you can still check on L1, the balance of the user 1 is still 0.

# You can use the `dry run` version if you just want to check the messages before actually sending them.
curl -H 'Content-Type: application/json' -d '{"dry_run": true}' http://127.0.0.1:5050/postman/flush
```
```json
{
    "messages_to_l1": [
        {
            "l2_contract_address":"0x34ba56f92265f0868c57d3fe72ecab144fc96f97954bbbc4252cef8e8a979ba",
            "l1_contract_address":"0xe7f1725e7734ce288f8367e1bb143e90bb3f0512",
            "payload":["0x0","0x1","0x1"]
        }
    ],
    "messages_to_l2":[],
    "generated_l2_transactions": [],
    "l1_provider":"dry run"
}
```
2. Actually flush the message to be sent on the L1 node.
```bash
# Flushing the message to actually send them to the L1.
curl -H 'Content-Type: application/json' -X POST http://127.0.0.1:5050/postman/flush
```
```json
{
    "messagesToL1": [
        {
            "l2_contract_address":"0x34ba56f92265f0868c57d3fe72ecab144fc96f97954bbbc4252cef8e8a979ba",
            "l1_contract_address":"0xe7f1725e7734ce288f8367e1bb143e90bb3f0512",
            "payload":["0x0","0x1","0x1"]
        }
    ],
    "messages_to_l2":[],
    "generated_l2_transactions": [],
    "l1_provider":"http://127.0.0.1:8545/"
}
```

### Etherum receive message and send message to L2
1. Now the message is received, we can consume it. You can try to run this command several time,
   you'll see the transaction reverting with `INVALID_MESSAGE_TO_CONSUME` once the message is consumed once. To consume the message, we have to provide it's content (balance of 1 to user 1).
```bash
cast send 0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512 "withdraw(uint256, uint256, uint256)" \
     0x34ba56f92265f0868c57d3fe72ecab144fc96f97954bbbc4252cef8e8a979ba 0x1 0x1 \
     --rpc-url $ETH_RPC_URL --private-key $ACCOUNT_PRIVATE_KEY \
     --gas-limit 999999
     
# We can now check the balance of user 1 on L1, it's 1.
cast call 0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512 "get_balance(uint256)(uint256)" 0x1
```
```bash
# output of send...

1
```

2. Let's now send back the amount 1 we just received to the user 1 on L2. As we will send a message, we need to provide at least 30k WEI.
```bash
cast send 0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512 "deposit(uint256, uint256, uint256)" \
     "$CONTRACT_L2" 0x1 0x1 \
     --rpc-url $ETH_RPC_URL --private-key $ACCOUNT_PRIVATE_KEY \
     --gas-limit 999999 --value 1gwei
     
# The balance is now 0 for the user 1 on ethereum.
cast call 0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512 "get_balance(uint256)(uint256)" 0x1
```
```bash
# output of send...

0
```

3. Flush the messages.
```bash
curl -H 'Content-Type: application/json' -X POST http://127.0.0.1:5050/postman/flush
```
```json
{
    "messagesToL1": [],
    "messagesToL2": [
        {
            "l2_contract_address":"...",
            "entry_point_selector":"0xc73f681176fc7b3f9693986fd7b14581e8d540519e27400e88b8713932be01",
            "l1_contract_address":"0xe7f1725e7734ce288f8367e1bb143e90bb3f0512",
            "payload":["0x1","0x1"],
            "paid_fee_on_l1":"0x3b9aca00",
            "nonce":"0x1"
        }
    ],
    "generated_l2_transactions": ["0x75337b9eb7f731226ba4ddea7a9c5b2f984ee9546c0cbb5d1c04e69f5d62aac"],
    "l1_provider":"http://127.0.0.1:8545/"
}
```
We can now check the balance of user 1 on L2, it's back to `0xff`.
```bash
starkli call "$CONTRACT_L2" get_balance 0x1
```
```json
[
    "0x00000000000000000000000000000000000000000000000000000000000000ff"
]
```

###  Mocking messages without running L1 node
1. Now, let's say we want to increase the balance of the user on L2 as if a message was sent from L1. Devnet has an endpoint `postman/send_message_to_l2` to mock a message coming from L1, without actually running an L1 node. Let's mock a message that sends the amount 2 to the user 1.
```bash
curl -H 'Content-Type: application/json' \
    -d '{"paid_fee_on_l1": "0x123", "l2_contract_address": '"$CONTRACT_L2"', "l1_contract_address": "0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512", "entry_point_selector": "0x00c73f681176fc7b3f9693986fd7b14581e8d540519e27400e88b8713932be01", "payload": ["0x1", "0x2"], "nonce": "0x1"}' \
    http://127.0.0.1:5050/postman/send_message_to_l2
```
```json
{
    "transaction_hash": "0x7f5c523f47bc88fa21f86ec4aaac8bbad69dafb43ae7072319dcec4d5d40af9"
}
```
The balance is now increased by 2, exactly as a message from L1 would have done.
```bash
starkli call "$CONTRACT_L2" get_balance 0x1
```
```json
[
    "0x0000000000000000000000000000000000000000000000000000000000000101"
]
```

2. Finally, to give an example of how to test a message sent by a Cairo contract without running the L1 node. Let's withdraw the amount 2 from the user 1.
```bash
# Withdraw to have the Cairo contract creating the message.
starkli invoke "$CONTRACT_L2" withdraw 0x1 0x2 0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512
```
Now that the message has been created by the Cairo contract, instead of using `flush` to send the message to L1 node, we can consume it manually and verify that the message has been correctly created by the Cairo contract:
```bash
curl -H 'Content-Type: application/json' \
    -d '{"from_address": "0x34ba56f92265f0868c57d3fe72ecab144fc96f97954bbbc4252cef8e8a979ba", "to_address": "0xe7f1725e7734ce288f8367e1bb143e90bb3f0512", "payload": ["0x0","0x1","0x2"]}' \
    http://127.0.0.1:5050/postman/consume_message_from_l2
```
```json
{
    "message_hash": "0x987b98434563ce4683f38c443d0c060492592960b525200ff7345d39c2f94fa2"
}
```
You can try to run the command again, and you'll see an error saying that the message has been totally consumed.

If we now check the balance of the user 1, it should be back to `0xff`.
```bash
starkli call "$CONTRACT_L2" get_balance 0x1
```
```json
[
    "0x00000000000000000000000000000000000000000000000000000000000000ff"
]
```

### Re-run with a script

To quickly setup the nodes for testing and re-run this exact sequence after restarting your nodes, you can use the following bash script:
```bash
bash run_e2e.sh
```
It's important to note that those operations must be done in this exact order to ensure that hard-coded addresses used in this guide are stil valid.
