FROM rust:1.70.0-slim-buster as builder

COPY . .

RUN cargo build --bin starknet-devnet --release

FROM debian:buster-slim

# Use tini to avoid hanging process on Ctrl+C
RUN apt-get -y update && \
    apt-get install tini && \
    apt-get autoremove -y && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Necessary for predeployed contracts
COPY crates/starknet/accounts_artifacts/ /crates/starknet/accounts_artifacts/
COPY --from=builder /target/release/starknet-devnet /usr/local/bin/starknet-devnet

ENTRYPOINT [ "tini", "--", "starknet-devnet", "--host", "0.0.0.0" ]
