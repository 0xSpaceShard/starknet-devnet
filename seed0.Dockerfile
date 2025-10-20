ARG BASE_IMAGE

FROM ${BASE_IMAGE}

ENTRYPOINT [ "tini", "--", "starknet-devnet", "--host", "0.0.0.0", "--seed", "0" ]
