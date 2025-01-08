# Contributing

To read about PR expectations, check out the [Pull requests](#pull-requests) section. To learn about setting up the project for development and testing, but also getting a per-feature insight, check out [Development](#development).

## Pull requests

> :warning: IMPORTANT NOTE :warning:
>
> All contributions are expected to be of the highest possible quality! That means the PR is thoroughly tested and documented, and without blindly generated ChatGPT code and documentation! PRs that do not comply with these rules stated here shall not be considered!

### Should you create a PR?

It is advised to [create an issue](https://github.com/0xSpaceShard/starknet-devnet-rs/issues/new/choose) before creating a PR. Creating an issue is the best way to reach somebody with repository-specific experience who can provide more info on how a problem/idea can be addressed and if a PR is needed.

### Checklist

The [PR template](pull_request_template.md) contains a checklist. It is important to go through the checklist to ensure the expected quality standards and to ensure the CI workflow succeeds once it is executed.

### Review

Once a PR is created, somebody from the team will review it. When a reviewer leaves a comment, the PR author should not mark the conversation as resolved. This is because the repository has a setting that prevents merging if there are unresolved conversations - let the reviewer resolve. The author can reply back with:

- a request for clarification from the reviewer
- a link to the commit which addresses the reviewer's observation (simply pasting the sha-digest is enough)

This is an example of a good author-reviewer correspondence: [link](https://github.com/0xSpaceShard/starknet-devnet-rs/pull/310#discussion_r1457142002).

#### Note to reviewers

This project's CI/CD platform (CircleCI) does not have the option to trigger the workflow on external PRs simply with a click. So once a PR is reviewed and looks like its workflow could pass, it can either be accepted & merged it blindly (which shall trigger the workflow on the target branch), or the following workaround can be used to trigger it:

```
# https://stackoverflow.com/questions/5884784/how-to-pull-remote-branch-from-somebody-elses-repo
$ git remote add <CONTRIBUTOR> <CONTRIBUTOR_GIT_FORK_URL>
$ git fetch <CONTRIBUTOR>
$ git checkout -b <CONTRIBUTOR>/<BRANCH> <CONTRIBUTOR>/<BRANCH>

$ git remote set-url --push <CONTRIBUTOR> git@github.com:0xSpaceShard/starknet-devnet-rs.git
$ git push <CONTRIBUTOR> HEAD
```

## Development

### Installation

Some developer scripts used in this project are written in Python 3, with dependencies specified in `scripts/requirements.txt`. You may want to [install the dependencies in a virtual environment](https://docs.python.org/3/library/venv.html#creating-virtual-environments).

Documentation maintenance requires installing `npm`.

### Visual Studio Code

It is highly recommended to get familiar with [Visual Studio Code Dev Containers](https://code.visualstudio.com/docs/devcontainers/create-dev-container#_dockerfile) and install [rust-analyzer](https://code.visualstudio.com/docs/languages/rust) extension.

### Linter

Run the linter with:

```
$ ./scripts/clippy_check.sh
```

### Formatter

Run the formatter with:

```
$ ./scripts/format.sh
```

If you encounter an error like

```
error: toolchain 'nightly-x86_64-unknown-linux-gnu' is not installed
```

Resolve it with:

```
$ rustup default nightly
```

### Unused dependencies

To check for unused dependencies, run:

```
$ ./scripts/check_unused_deps.sh
```

If you think this reports a dependency as a false positive (i.e. isn't unused), check [here](https://github.com/bnjbvr/cargo-machete#false-positives).

### Spelling check

To check for spelling errors in the code, run:

```
$ ./scripts/check_spelling.sh
```

If you think this reports a false-positive, check [here](https://crates.io/crates/typos-cli#false-positives).

### Pre-commit

To speed up development, you can put the previous steps (and more) in a local script defined at `.git/hooks/pre-commit` to have it run before each commit ([more info](https://git-scm.com/book/en/v2/Customizing-Git-Git-Hooks)).

### Testing

#### Prerequisites

Some tests require the `anvil` command, so you need to [install Foundry](https://book.getfoundry.sh/getting-started/installation). The `anvil` command might not be usable by tests if you run them using VS Code's `Run Test` button available just above the test case. Either run tests using a shell which has foundry/anvil in `PATH`, or modify the BackgroundAnvil Command to specify `anvil` by its path on your system.

To ensure that integration tests pass, be sure to have run `cargo build --release` or `cargo run --release` prior to testing. This builds the production target used in integration tests, so spawning BackgroundDevnet won't time out.

#### Test execution

Run all tests using all available CPUs with:

```
$ cargo test
```

#### Benchmarking

To test if your contribution presents an improvement in execution time, check out the script at `scripts/benchmark/command_stat_test.py`.

### Updating OpenZeppelin contracts

Tests in devnet require an erc20 contract with the `Mintable` feature, keep in mind that before the compilation process of [cairo-contracts](https://github.com/OpenZeppelin/cairo-contracts/) you need to mark the `Mintable` check box in this [wizard](https://wizard.openzeppelin.com/cairo) and copy this implementation to `/src/presets/erc20.cairo`.

If smart contract constructor logic has changed, Devnet's predeployment logic needs to be changed, e.g. `simulate_constructor` in `crates/starknet-devnet-core/src/account.rs`.

### Updating Starknet

Updating the underlying Starknet is done by updating the `blockifier` dependency. It also requires updating the `STARKNET_VERSION` constant.

### Updating JSON-RPC API

Updating the RPC requires following the specification files in the [starknet-specs repository](https://github.com/starkware-libs/starknet-specs). The spec_reader testing utility requires these files to be copied into the Devnet repository. The `RPC_SPEC_VERSION` constant needs to be updated accordingly.

### Adding new dependencies

When adding new Rust dependencies, specify them in the root Cargo.toml and use `{ workspace = true }` in crate-specific Cargo.toml files.

### Updating documentation

The documentation website content has [its own readme](../website/README.md).

### New Devnet version release

To release a new version, check out the [release docs](../RELEASE.md).
