#!/bin/bash

set -eu

trap 'for killable in $(jobs -p); do kill $killable; done' EXIT

HOST=localhost
PORT1=5049
DEVNET1_URL="http://$HOST:$PORT1"
PORT2=5050
DEVNET2_URL="http://$HOST:$PORT2"


poetry run starknet-devnet --host "$HOST" --port "$PORT1" --seed 42 --accounts 1 --hide-predeployed-accounts &
DEVNET1_PID=$!
curl --retry 20 --retry-delay 1 --retry-connrefused -s -o /dev/null "$DEVNET1_URL/is_alive"
echo "Started up devnet1; pid: $DEVNET1_PID"

poetry run starknet-devnet --host "$HOST" --port "$PORT2" --fork-network "$DEVNET1_URL" --accounts 0 &
DEVNET2_PID=$!
curl --retry 20 --retry-delay 1 --retry-connrefused -s -o /dev/null "$DEVNET2_URL/is_alive"
echo "Started up devnet2; pid: $DEVNET2_PID"

# # get public key of predeployed account
# for port in "$PORT1" "$PORT2"; do
#     echo "Polling devnet at :$port"
#     poetry run starknet get_storage_at \
#         --feeder_gateway_url "http://$HOST:$port" \
#         --contract_address 0x347be35996a21f6bf0623e75dbce52baba918ad5ae8d83b6f416045ab22961a \
#         --key 550557492744938365112574611882025123252567779123164597803728068558738016655
# done

source ~/venvs/cairo_venv-0.10.1-pre/bin/activate
DEPLOYMENT_URL="$DEVNET1_URL"
starknet deploy \
    --contract test/artifacts/contracts/cairo/contract.cairo/contract.json \
    --inputs 10 \
    --gateway_url "$DEPLOYMENT_URL" \
    --feeder_gateway_url "$DEPLOYMENT_URL" \
    --salt 0x99 \
    --no_wallet
echo "Deployed contract"
CONTRACT_ADDRESS=0x07c80f5573d4c636960b56b02a01514d487c6e6a2c6f9242490280c932a32f71

export STARKNET_WALLET="starkware.starknet.wallets.open_zeppelin.OpenZeppelinAccount"
ACCOUNT_DIR="."
rm -rf ./starknet_open_zeppelin_accounts.json*

starknet new_account --network alpha-goerli --account_dir "$ACCOUNT_DIR"

ACCOUNT_DEPLOYMENT_URL="$DEVNET1_URL"
starknet deploy_account \
    --gateway_url "$ACCOUNT_DEPLOYMENT_URL" \
    --feeder_gateway_url "$ACCOUNT_DEPLOYMENT_URL" \
    --network alpha-goerli \
    --account_dir "$ACCOUNT_DIR" \
    --max_fee 0
echo "Deployed account"

INVOKE_URL="$DEVNET1_URL"
INVOKE_HASH=$(starknet invoke \
    --abi test/artifacts/contracts/cairo/contract.cairo/contract_abi.json \
    --function increase_balance \
    --inputs 10 20 \
    --address "$CONTRACT_ADDRESS" \
    --gateway_url "$INVOKE_URL" \
    --feeder_gateway_url "$INVOKE_URL" \
    --max_fee 0 \
    --network_id alpha-goerli \
    --chain_id 0x534e5f474f45524c49 \
    --account_dir "$ACCOUNT_DIR" | sed -rn 's/^Transaction hash: (.*)$/\1/p'
)
echo "Invoked contract on $INVOKE_URL"

echo "Transaction on $DEVNET1_URL"
starknet get_transaction --hash "$INVOKE_HASH" --feeder_gateway_url "$DEVNET1_URL"
echo "Transaction on $DEVNET2_URL"
starknet get_transaction --hash "$INVOKE_HASH" --feeder_gateway_url "$DEVNET2_URL"
echo "Block on $DEVNET2_URL"
starknet get_block --feeder_gateway "$DEVNET2_URL"

for url in "$DEVNET1_URL" "$DEVNET2_URL"; do
    starknet call \
        --abi test/artifacts/contracts/cairo/contract.cairo/contract_abi.json \
        --feeder_gateway_url "$url" \
        --address "$CONTRACT_ADDRESS" \
        --function get_balance
done
