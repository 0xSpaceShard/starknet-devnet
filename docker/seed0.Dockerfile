ARG BASE_TAG

FROM shardlabs/starknet-devnet-rs:${BASE_TAG}

ENTRYPOINT [ "tini", "--", "starknet-devnet", "--host", "0.0.0.0", "--seed", "0" ]
