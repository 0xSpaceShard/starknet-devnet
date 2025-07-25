#!/bin/bash

set -eu

N=$1

for _ in $(seq 1 $N); do
    curl localhost:5050/ -w "\n" -sSf --json '{
        "jsonrpc": "2.0",
        "id": 1,
        "method": "devnet_mint",
        "params": {
            "amount": 1,
            "address": "0x1"
        }
    }'
done
