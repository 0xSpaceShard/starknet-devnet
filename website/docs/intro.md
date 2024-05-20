---
sidebar_position: 1
---

<!-- TODO: add testnet difference or other general disclaimers -->
<!-- TODO: add instructions for editing docs -->
<!-- TODO: add examples:
  - L1-L2 - use content of contracts/README.md, add section in postman.md that mentions the example and starknet-hardhat-plugin; consider adding developer section to postman.md
 -->


## Lite Mode

Runs Devnet in a minimal lite mode by just skipping the block hash calculation. This is useful for testing purposes when the block hash is not needed.

```
$ starknet-devnet --lite-mode
```

## Forking

To interact with contracts deployed on mainnet or testnet, you can use the forking to simulate the origin and experiment with it locally, making no changes to the origin itself.

```
$ starknet-devnet --fork-network <URL> [--fork-block <BLOCK_NUMBER>]
```

The value passed to `--fork-network` should be the URL to a Starknet JSON-RPC API provider. Specifying a `--fork-block` is optional; it defaults to the `"latest"` block at the time of Devnet's start-up. All calls will first try Devnet's state and then fall back to the forking block.

## Querying old state by specifying block hash or number

With state archive capacity set to `full`, Devnet will store full state history. The default mode is `none`, where no old states are stored.

```
$ starknet-devnet --state-archive-capacity <CAPACITY>
```

All RPC endpoints that support querying the state at an old (non-latest) block only work with state archive capacity set to `full`.

## Development

### Installation

Some developer scripts used in this project are written in Python 3, with dependencies specified in `scripts/requirements.txt`. You may want to [install the dependencies in a virtual environment](https://docs.python.org/3/library/venv.html#creating-virtual-environments).

### Development - Visual Studio Code

It is highly recommended to get familiar with [Visual Studio Code Dev Containers](https://code.visualstudio.com/docs/devcontainers/create-dev-container#_dockerfile) and install [rust-analyzer](https://code.visualstudio.com/docs/languages/rust) extension.

### Development - Linter

Run the linter with:

```
./scripts/clippy_check.sh
```

### Development - Formatter

Run the formatter with:

```
./scripts/format.sh
```

If you encounter an error like

```
error: toolchain 'nightly-x86_64-unknown-linux-gnu' is not installed
```

Resolve it with:

```
rustup default nightly
```

### Development - Unused dependencies

To check for unused dependencies, run:

```
./scripts/check_unused_deps.sh
```

If you think this reports a dependency as a false positive (i.e. isn't unused), check [here](https://github.com/bnjbvr/cargo-machete#false-positives).

### Development - Spelling check

To check for spelling errors in the code, run:

```
./scripts/check_spelling.sh
```

If you think this reports a false-positive, check [here](https://crates.io/crates/typos-cli#false-positives).

### Development - pre-commit

To speed up development, you can put all the previous steps (and more) in a script defined at [.git/hooks/pre-commit](https://git-scm.com/book/en/v2/Customizing-Git-Git-Hooks).

### Development - Testing

#### Prerequisites

Some tests require the `anvil` command, so you need to [install Foundry](https://book.getfoundry.sh/getting-started/installation). The `anvil` command might not be usable by tests if you run them using VS Code's `Run Test` button available just above the test case. Either run tests using a shell which has foundry/anvil in `PATH`, or modify the BackgroundAnvil Command to specify `anvil` by its path on your system.

To ensure that integration tests pass, be sure to have run `cargo build --release` or `cargo run --release` prior to testing. This builds the production target used in integration tests, so spawning BackgroundDevnet won't time out.

#### Test execution

Run all tests using all available CPUs with:

```
$ cargo test
```

The previous command might cause your testing to die along the way due to memory issues. In that case, limiting the number of jobs helps, but depends on your machine (rule of thumb: N=6):

```
$ cargo test --jobs <N>
```

#### Benchmarking

To test if your contribution presents an improvement in execution time, check out the script at `scripts/benchmark/command_stat_test.py`.

##### Cargo Bench execution

To run the criterion benchmarks and generate a performance report:

```
$ cargo bench
```

This command will compile the benchmarks and run them using all available CPUs on your machine. Criterion will perform multiple iterations of each benchmark to collect performance data and generate statistical analysis.

Check the report created at `target/criterion/report/index.html`

Criterion is highly configurable and offers various options to customise the benchmarking process. You can find more information about Criterion and its features in the [Criterion documentation](https://bheisler.github.io/criterion.rs/book/index.html).

To measure and benchmark memory it is best to use external tools such as Valgrind, Leaks, etc.

### Development - Docker

Due to internal needs, images with arch suffix are built and pushed to Docker Hub, but this is not mentioned in the user docs as users should NOT be needing it.

This is what happens under the hood on `main`:

- build `shardlabs/starknet-devnet-rs-<COMMIT_SHA1>-amd`
- build `shardlabs/starknet-devnet-rs-<COMMIT_SHA1>-arm`
- create and push joint docker manifest called `shardlabs/starknet-devnet-rs-<COMMIT_SHA1>`
  - same for `latest`

### Development - Updating OpenZeppelin contracts

Tests in devnet require an erc20 contract with the `Mintable` feature, keep in mind that before the compilation process of [cairo-contracts](https://github.com/OpenZeppelin/cairo-contracts/) you need to mark the `Mintable` check box in this [wizard](https://wizard.openzeppelin.com/cairo) and copy this implementation to `/src/presets/erc20.cairo`.

If smart contract constructor logic has changed, Devnet's predeployment logic needs to be changed, e.g. `simulate_constructor` in `crates/starknet-devnet-core/src/account.rs`.

### Development - Updating Starknet

Updating the underlying Starknet is done by updating the `blockifier` dependency. It also requires updating the `STARKNET_VERSION` constant.

### Development - Updating JSON-RPC API

Updating the RPC requires following the specification files in the [starknet-specs repository](https://github.com/starkware-libs/starknet-specs). The spec_reader testing utility requires these files to be copied into the Devnet repository. The `RPC_SPEC_VERSION` constant needs to be updated accordingly.

### Development - New Devnet version release

To release a new version, follow these steps:

1. Increment the semver in Cargo.toml of those Devnet crates that have changed. Use `scripts/check_crate_changes.sh` for this. Preferably create a separate PR for the increment, such as [this one](https://github.com/0xSpaceShard/starknet-devnet-rs/pull/398).

2. The publishing of crates and Docker images is done automatically in CI when merged into the main branch.

3. When the CI workflow is done, create a git tag of the form `vX.Y.Z`, push it and create a GitHub release with notes describing changes since the last release.

4. Attach the [binary artifacts built in CI](https://circleci.com/docs/artifacts/#artifacts-overview) to the release. Use `scripts/fetch_ci_binaries.py` to fetch all artifacts of a CI workflow.

### Development - External PRs

Read more about how to review PRs in [the guidelines](.github/CONTRIBUTING.md#review).

Our CI/CD platform (CircleCI) does not have the option to trigger the workflow on external PRs with a simple click. So once a PR is reviewed and looks like its workflow could pass, you can either accept & merge it blindly (which shall trigger the workflow on the target branch), or use the following workaround to trigger it:

```
# https://stackoverflow.com/questions/5884784/how-to-pull-remote-branch-from-somebody-elses-repo
$ git remote add <CONTRIBUTOR> <CONTRIBUTOR_GIT_FORK_URL>
$ git fetch <CONTRIBUTOR>
$ git checkout -b <CONTRIBUTOR>/<BRANCH> <CONTRIBUTOR>/<BRANCH>

$ git remote set-url --push <CONTRIBUTOR> git@github.com:0xSpaceShard/starknet-devnet-rs.git
$ git push <CONTRIBUTOR> HEAD
```

## ✏️ Contributing

We ❤️ and encourage all contributions!

[Click here](.github/CONTRIBUTING.md) for the development guide.

## 🙌 Special Thanks

Special thanks to all the [contributors](https://github.com/0xSpaceShard/starknet-devnet-rs/graphs/contributors)!
