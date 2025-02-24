<!-- logo / title -->
<p align="center" style="margin-bottom: 0px !important">
  <img width="200" src="https://github.com/0xSpaceShard/starknet-devnet-rs/assets/21069052/4791b0e4-58fc-4a44-8f87-fc0db636a5c7" alt="Devnet-RS" align="center">
</p>

<h1 align="center" style="margin-top: 12px !important">Starknet Devnet RS</h1>

<p align="center" dir="auto">
  <a href="https://crates.io/crates/starknet-devnet" target="_blank">
    <img src="https://img.shields.io/crates/v/starknet-devnet?color=yellow" style="max-width: 100%;">
  </a>
  <a href="https://hub.docker.com/r/shardlabs/starknet-devnet-rs/tags" target="_blank">
    <img src="https://img.shields.io/badge/dockerhub-images-important.svg?logo=Docker" style="max-width: 100%;">
  </a>
  <a href="https://starkware.co/" target="_blank">
    <img src="https://img.shields.io/badge/powered_by-StarkWare-navy" style="max-width: 100%;">
  </a>
</p>

## 📖 About

Starknet Devnet RS is a local testnet for Starknet written in Rust. This tool is designed for developers who want to test their smart contracts and applications in a local environment that closely resembles the actual Starknet network.

## 🚀 Quick Start

```bash
# Install via Cargo
cargo install starknet-devnet

# Or use Docker
docker pull shardlabs/starknet-devnet-rs:latest
docker run -p 5050:5050 shardlabs/starknet-devnet-rs:latest
```

## 💻 System Requirements

- Rust 1.65 or higher
- Cargo
- Git
- Docker (optional)

## ✨ Features

- [Forking](https://0xspaceshard.github.io/starknet-devnet-rs/docs/forking) - interact with contracts deployed on mainnet or testnet
- [Account Impersonation](https://0xspaceshard.github.io/starknet-devnet-rs/docs/account-impersonation) - test with different accounts
- [L1-L2 Interaction](https://0xspaceshard.github.io/starknet-devnet-rs/docs/postman) - test cross-layer communication
- [Predeployed Contracts](https://0xspaceshard.github.io/starknet-devnet-rs/docs/predeployed) - accounts, tokens, and more
- [Block Manipulation](https://0xspaceshard.github.io/starknet-devnet-rs/docs/blocks) - creation, abortion, and more
- [Time Manipulation](https://0xspaceshard.github.io/starknet-devnet-rs/docs/starknet-time/) - network time control
- [State Management](https://0xspaceshard.github.io/starknet-devnet-rs/docs/dump-load-restart) - dump, load, and restart state
- [Flexible Configuration](https://0xspaceshard.github.io/starknet-devnet-rs/docs/running/cli) - customize according to your needs

## 🌐 Documentation

Complete documentation is available [here](https://0xspaceshard.github.io/starknet-devnet-rs/).

### Documentation Structure:
- [Getting Started Guide](https://0xspaceshard.github.io/starknet-devnet-rs/docs/getting-started)
- [API Reference](https://0xspaceshard.github.io/starknet-devnet-rs/docs/api)
- [Usage Examples](https://0xspaceshard.github.io/starknet-devnet-rs/docs/examples)
- [Frequently Asked Questions](https://0xspaceshard.github.io/starknet-devnet-rs/docs/faq)

## 📦 starknet-devnet-js

Simplify the installation, spawning, and usage of Devnet in your tests by using the official JavaScript wrapper. Learn more [here](https://github.com/0xspaceShard/starknet-devnet-js).

## 🤝 How to Contribute

We ❤️ and welcome all contributions! Here's how you can help:

1. Fork the repository
2. Create a branch for your changes
3. Make your changes and test them
4. Create a pull request with a description of your changes

Please read our [development guide](.github/CONTRIBUTING.md) before getting started.

## 📝 License

This project is licensed under the MIT License.
