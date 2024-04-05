FROM rust:1.74.0-slim-buster as builder

ARG TARGET

COPY . .

# TODO this won't work on main, but perhaps we can change the whole image building scheme
# by building a binary outside, publishing it on github, and copying it into an image
# Alternatively just return the libssl dependency - if that will be compatible with other targets
RUN apt-get update && \
    apt-get install make -y

RUN cargo build --bin starknet-devnet --release --target=${TARGET}

FROM debian:buster-slim

ARG TARGET

# Use tini to avoid hanging process on Ctrl+C
# Use ca-certificates to allow forking from URLs using https scheme
RUN apt-get -y update && \
    apt-get install tini && \
    apt-get install ca-certificates -y && \
    apt-get autoremove -y && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /target/${TARGET}/release/starknet-devnet /usr/local/bin/starknet-devnet

ENTRYPOINT [ "tini", "--", "starknet-devnet", "--host", "0.0.0.0" ]
