ARG BASE_TAG

FROM shardlabs/starknet-devnet:${BASE_TAG}

ENTRYPOINT [ "starknet-devnet", "--host", "0.0.0.0", "--port", "5050", "--seed", "0" ]
