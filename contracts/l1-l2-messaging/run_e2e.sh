#!/bin/bash

# Bash script to run E2E example all at once.
source ./.env

# Compiles contracts to ensure latest version.
scarb --manifest-path ./cairo/Scarb.toml build
forge install --root ./solidity
forge build --root ./solidity

# Deploy mock messaging contract on L1.
curl -H 'Content-Type: application/json' \
     -d '{"network_url": "http://127.0.0.1:8545"}' \
     http://127.0.0.1:5050/postman/load_l1_messaging_contract

# Deploy L1L2 contract on L1.
pushd ./solidity
forge script ./script/L1L2.s.sol:Deploy --broadcast --rpc-url $ETH_RPC_URL
popd

# Declare and deploy l1-l2 contract on L2.
starkli declare ./cairo/target/dev/cairo_l1_l2.contract_class.json
starkli deploy 0x0211fd0483be230ba40d43f51bd18ae239b913f529f95ce10253e514175efb3e --salt 123

# Add some balance and check it.
starkli invoke 0x03c80468c8fe2fd36fadf1b484136b4cd8a372f789e8aebcc6671e00101290a4 increase_balance 0x1 0xff

# Issue a withdraw to send message to L1 with amount 1 for user 1.
starkli invoke 0x03c80468c8fe2fd36fadf1b484136b4cd8a372f789e8aebcc6671e00101290a4 withdraw 0x1 1 0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512

# Flush messages to actually send the message to L1.
curl -H 'Content-Type: application/json' -X POST http://127.0.0.1:5050/postman/flush

# Consume the message on L1.
cast send 0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512 "withdraw(uint256, uint256, uint256)" \
     0x34ba56f92265f0868c57d3fe72ecab144fc96f97954bbbc4252cef8e8a979ba 0x1 0x1 \
     --rpc-url $ETH_RPC_URL --private-key $ACCOUNT_PRIVATE_KEY \
     --gas-limit 999999

# Send back the balance of 1 to L2 user.
cast send 0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512 "deposit(uint256, uint256, uint256)" \
     0x03c80468c8fe2fd36fadf1b484136b4cd8a372f789e8aebcc6671e00101290a4 0x1 0x1 \
     --rpc-url $ETH_RPC_URL --private-key $ACCOUNT_PRIVATE_KEY \
     --gas-limit 999999 --value 1gwei

# Flush messages to actually send message to L2.
curl -H 'Content-Type: application/json' -X POST http://127.0.0.1:5050/postman/flush

# Simulate message from L1 to increase the balance.
curl -H 'Content-Type: application/json' \
    -d '{"paid_fee_on_l1": "0x123", "l2_contract_address": "0x03c80468c8fe2fd36fadf1b484136b4cd8a372f789e8aebcc6671e00101290a4", "l1_contract_address": "0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512", "entry_point_selector": "0x00c73f681176fc7b3f9693986fd7b14581e8d540519e27400e88b8713932be01", "payload": ["0x1", "0x2"], "nonce": "0x1"}' \
    http://127.0.0.1:5050/postman/send_message_to_l2

# Send back some balance to consume manually.
starkli invoke 0x03c80468c8fe2fd36fadf1b484136b4cd8a372f789e8aebcc6671e00101290a4 withdraw 0x1 1 0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512

curl -H 'Content-Type: application/json' \
    -d '{"from_address": "0x34ba56f92265f0868c57d3fe72ecab144fc96f97954bbbc4252cef8e8a979ba", "to_address": "0xe7f1725e7734ce288f8367e1bb143e90bb3f0512", "payload": ["0x0","0x1","0x1"]}' \
    http://127.0.0.1:5050/postman/consume_message_from_l2


