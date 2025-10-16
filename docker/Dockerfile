FROM rust:1.86-alpine3.21 AS builder

RUN apk update && \
    apk add --no-cache pkgconfig make musl-dev openssl-dev perl

WORKDIR /app

COPY Cargo.toml Cargo.lock rust-toolchain.toml ./

COPY crates/starknet-devnet/Cargo.toml ./crates/starknet-devnet/
COPY crates/starknet-devnet-core/Cargo.toml ./crates/starknet-devnet-core/
COPY crates/starknet-devnet-server/Cargo.toml ./crates/starknet-devnet-server/
COPY crates/starknet-devnet-types/Cargo.toml ./crates/starknet-devnet-types/
COPY tests/integration/Cargo.toml ./tests/integration/

RUN for crate in starknet-devnet-core starknet-devnet-server starknet-devnet-types; do \
      mkdir -p crates/$crate/src; \
      echo 'pub fn dummy() {}' > crates/$crate/src/lib.rs; \
    done

RUN mkdir -p crates/starknet-devnet/src && echo 'fn main() {}' > crates/starknet-devnet/src/main.rs

RUN mkdir -p tests/integration/src && echo 'pub fn dummy() {}' > tests/integration/src/lib.rs

RUN cargo build --bin starknet-devnet --release

RUN rm -rf ./src ./crates/*/src ./tests/integration/src

COPY . .

RUN cargo build --bin starknet-devnet --release

FROM alpine:3.21

RUN apk add --no-cache tini ca-certificates

COPY --from=builder /target/release/starknet-devnet /usr/local/bin/starknet-devnet

EXPOSE 5050

ENTRYPOINT [ "tini", "--", "starknet-devnet", "--host", "0.0.0.0" ]
