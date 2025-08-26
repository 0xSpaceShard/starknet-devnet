FROM rust:1.86.0-slim-bookworm AS builder

COPY . .

RUN apt-get -y update && \
    apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    gnupg2 \
    curl \
    ca-certificates \
    libssl-dev \
    lsb-release \
    software-properties-common \
    zstd \
    lld \
    wget \
    zlib1g-dev

# Add official LLVM APT repo and install LLVM 19
RUN curl -fsSL https://apt.llvm.org/llvm.sh -o llvm.sh && \
    chmod +x llvm.sh && \
    ./llvm.sh 19 all && \
    rm llvm.sh

RUN apt-get install -y \
    libgmp3-dev \
    libmlir-19-dev \
    libpolly-19-dev \
    libzstd-dev \
    mlir-19-tools

RUN ln -s /usr/bin/llvm-config-19 /usr/local/bin/llvm-config

RUN cargo build --bin starknet-devnet --release

FROM debian:bookworm-slim

# Use tini to avoid hanging process on Ctrl+C
# Use ca-certificates to allow forking from URLs using https scheme
RUN apt-get -y update && \
    apt-get install -y --no-install-recommends \
    tini \
    ca-certificates \
    autoremove \
    clean \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /target/release/starknet-devnet /usr/local/bin/starknet-devnet

# The default port; exposing is beneficial if using Docker GUI
EXPOSE 5050

ENTRYPOINT [ "tini", "--", "starknet-devnet", "--host", "0.0.0.0" ]
