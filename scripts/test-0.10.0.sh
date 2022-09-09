#!/bin/bash

set -eu

export STARKNET_WALLET=starkware.starknet.wallets.open_zeppelin.OpenZeppelinAccount

CONTRACTS_DIR="test/artifacts/contracts/cairo"

source ~/.cache/pypoetry/virtualenvs/starknet-devnet-fpM8jv2w-py3.8/bin/activate

# ACCOUNT_DIR=~/.starknet_accounts
ACCOUNT_DIR=starknet_devnet/accounts_artifacts/starknet_cli_wallet
ACCOUNT_FILE="$ACCOUNT_DIR"/starknet_open_zeppelin_accounts.json

# starknet deploy_account \
#     --gateway_url http://localhost:5050 \
#     --feeder_gateway_url http://localhost:5050 \
#     --network alpha-goerli \
#     --account_dir $ACCOUNT_DIR
# echo "Deployed account"

ACCOUNT_ADDRESS=$(jq -r '.["alpha-goerli"].__default__.address' $ACCOUNT_FILE)
echo "Account address: $ACCOUNT_ADDRESS"
echo "Account class hash:"
starknet get_class_hash_at --contract_address $ACCOUNT_ADDRESS --feeder_gateway_url http://localhost:5050

starknet declare \
    --contract "$CONTRACTS_DIR/contract.cairo/contract.json" \
    --gateway_url http://localhost:5050 \
    --feeder_gateway_url http://localhost:5050 \
    --network alpha-goerli \
    --account_dir $ACCOUNT_DIR
echo "Class declared"

starknet deploy \
    --contract "$CONTRACTS_DIR/contract.cairo/contract.json" \
    --inputs 10 \
    --gateway_url http://localhost:5050 \
    --no_wallet \
    --salt 0x42
CONTRACT_ADDRESS="0x01e3d46cb9a1498da62885f9467cee4036103a46be4259a224df697ad06aa7a9"
echo "Deployed contract"

# minting no longer needed since the account is predeployed and prefunded
# curl localhost:5050/mint \
#     -H "Content-Type: application/json" \
#     -d "{ \"address\": \"$ACCOUNT_ADDRESS\", \"amount\": 1000000000000000000, \"lite\": true }"

starknet call \
    --gateway_url http://localhost:5050 \
    --feeder_gateway_url http://localhost:5050 \
    --abi "$CONTRACTS_DIR/contract.cairo/contract_abi.json" \
    --address $CONTRACT_ADDRESS \
    --function get_balance \
    --account_dir $ACCOUNT_DIR
echo "Called"

starknet invoke --estimate_fee \
    --abi "$CONTRACTS_DIR/contract.cairo/contract_abi.json" \
    --function increase_balance \
    --inputs 10 20 \
    --gateway_url http://localhost:5050 \
    --feeder_gateway_url http://localhost:5050 \
    --address $CONTRACT_ADDRESS \
    --network_id alpha-goerli \
    --chain_id 0x534e5f474f45524c49 \
    --account_dir $ACCOUNT_DIR
echo "Estimated fee"

starknet invoke \
    --abi "$CONTRACTS_DIR/contract.cairo/contract_abi.json" \
    --function increase_balance \
    --inputs 10 20 \
    --gateway_url http://localhost:5050 \
    --feeder_gateway_url http://localhost:5050 \
    --address $CONTRACT_ADDRESS \
    --network_id alpha-goerli \
    --chain_id 0x534e5f474f45524c49 \
    --account_dir $ACCOUNT_DIR
echo "Invoked"

starknet call \
    --gateway_url http://localhost:5050 \
    --feeder_gateway_url http://localhost:5050 \
    --abi "$CONTRACTS_DIR/contract.cairo/contract_abi.json" \
    --address $CONTRACT_ADDRESS \
    --function get_balance \
    --account_dir $ACCOUNT_DIR
echo "Called"
