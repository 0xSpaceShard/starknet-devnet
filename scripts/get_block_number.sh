#!/bin/bash

set -eu

curl localhost:5050/rpc -X POST -H "Content-Type: application/json" --data '{
        "jsonrpc":"2.0",
        "method":"starknet_blockNumber",
        "id":1
}'
