FROM rust:1.87-alpine3.21 AS builder

COPY . .

RUN apk update && \
    apk add --no-cache pkgconfig make musl-dev openssl-dev perl

RUN cargo build --bin starknet-devnet --release

FROM alpine:3.21

# Use tini to avoid hanging process on Ctrl+C
# Use ca-certificates to allow forking from URLs using https scheme
RUN apk add --no-cache tini ca-certificates

COPY --from=builder /target/release/starknet-devnet /usr/local/bin/starknet-devnet

# The default port; exposing is beneficial if using Docker GUI
EXPOSE 5050

ENTRYPOINT [ "tini", "--", "starknet-devnet", "--host", "0.0.0.0" ]
