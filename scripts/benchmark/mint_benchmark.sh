#!/bin/bash

set -eu

N=$1
ADDRESS=$2

for _ in $(seq 1 $N); do
    curl -w "\n" -sSf -H "Content-Type: application/json" -d '{
        "amount": 1,
        "address": "'"$ADDRESS"'"
    }' localhost:5050/mint
done