FROM rust:1.69.0-slim-buster as builder

COPY . .
RUN cargo build --bin starknet-devnet --release

FROM debian:buster-slim

COPY crates/starknet/accounts_artifacts/ /crates/starknet/accounts_artifacts/
COPY --from=builder /target/release/starknet-devnet /usr/local/bin/starknet-devnet

ENTRYPOINT [ "starknet-devnet", "--host", "0.0.0.0", "--port", "5050" ]
