FROM rust:1.76.0-slim-buster as builder

COPY . .

RUN apt-get -y update && \
    apt-get install pkg-config -y && \
    apt-get install make -y

RUN cargo build --bin starknet-devnet --release

FROM debian:buster-slim

# Use tini to avoid hanging process on Ctrl+C
# Use ca-certificates to allow forking from URLs using https scheme
RUN apt-get -y update && \
    apt-get install tini && \
    apt-get install ca-certificates -y && \
    apt-get autoremove -y && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /target/release/starknet-devnet /usr/local/bin/starknet-devnet

ENTRYPOINT [ "tini", "--", "starknet-devnet", "--host", "0.0.0.0" ]
