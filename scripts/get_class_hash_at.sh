#!/bin/bash

set -eu

curl localhost:5050/rpc -X POST -H "Content-Type: application/json" --data '{
        "jsonrpc":"2.0",
        "method":"starknet_getClassHashAt",
        "params":{
            "block_id": { "block_hash": "0x1" },
            "contract_address": "0x64b48806902a367c8598f4f95c305e8c1a1acba5f082d294a43793113115691"
        },
        "id":1
}'
