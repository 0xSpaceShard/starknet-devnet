FROM debian:buster-slim

ARG BIN_PATH

# Use tini to avoid hanging process on Ctrl+C
# Use ca-certificates to allow forking from URLs using https scheme
RUN apt-get -y update && \
    apt-get install tini && \
    apt-get install ca-certificates -y && \
    apt-get autoremove -y && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

COPY ${BIN_PATH} /usr/local/bin/starknet-devnet

ENTRYPOINT [ "tini", "--", "starknet-devnet", "--host", "0.0.0.0" ]
