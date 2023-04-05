---
sidebar_position: 18
---

# Development

If you're a developer willing to contribute, be sure to have installed [**Poetry**](https://pypi.org/project/poetry/) and all the dependency packages by running the installation script. Prerequisites for running the script: `gcc`, `g++`, `gmp`, `npm`.

To use an existing Cairo 1 compiler repository, set the environment variable `CAIRO_1_COMPILER_MANIFEST` to the path of the `Cargo.toml` of the compiler. If this variable is not set, the installation script will download a new compiler repository as a subdirectory of your starknet-devnet directory. This compiler downloading is mainly intended to be used by the CI/CD pipeline, but developers can locally depend on it if they want to.

```bash
$ ./scripts/install_dev_tools.sh
```

## Development - Run

```text
$ poetry run starknet-devnet
```

## Development - Run in debug mode

This will restart Devnet on each code change:

```text
$ ./scripts/starknet_devnet_debug.sh
```

## Development - Format and Lint

```text
$ ./scripts/format.sh
$ ./scripts/lint.sh
```

## Development - Test

When running tests locally, do it from the project root.

First generate the artifacts:

```bash
$ ./scripts/compile_contracts.sh
```

To run the tests, use one of these commands:

```bash
$ ./scripts/test.sh [TEST_CASE] # parallelized testing - using auto detected number of CPU cores

$ poetry run pytest -s -v test/ # for more verbose output

$ poetry run pytest test/<TEST_FILE> # for a single file

$ poetry run pytest test/<TEST_FILE>::<TEST_CASE> # for a single test case
```

:::info

To enable testing of custom recompilation, set the environment variable `CAIRO_1_COMPILER_MANIFEST` to the path of the Cargo.toml file of your local Cairo 1 compiler. If you do not set this variable, that test group will fail.

:::

:::info

If you are experiencing test failures on macOS related to `fork()` command
consider running `export OBJC_DISABLE_INITIALIZE_FORK_SAFETY=YES` in your shell.

:::

## Development - Check versioning consistency

```
$./scripts/check_versions.sh
```

## Development - Adapting to a new version of cairo-lang

Install the exact version with `poetry add cairo-lang@<VERSION>`.

If you only know the URL of the archive (e.g. ZIP) of a new cairo-lang version, you can install it with

```
$ poetry add <URL>
```

After adding a new cairo-lang version, you will probably want to recompile contract artifacts.

Why are we installing the exact version? We depend on Starknet's internal code, and any minor change might make Devnet unusable.

When adapting to a new cairo-lang version, to make the tests pass, some hashes will need to be replaced (at least a different version string is stored in the compilation artifacts, leading to a different hash). This is the main argument for keeping the hardcoded hash values in tests: they are only expected to change if a new version cairo-lang version is being added or there is a change with the smart contracts themselves, otherwise a change in the expected hash values probably indicates a bug.

## Development - Updating accounts

1. Set up https://github.com/OpenZeppelin/cairo-contracts/ locally

   - `git clone ... && pip install cairo-nile && nile init`

2. `git checkout` to desired version
3. `nile compile --directory src`
4. Copy and minify `artifacts/Account.json` and `artifacts/abi/Account.json`
5. Update the precalculated hash

   - Predeployed account addresses should be intact

6. Update directory/file names containing the version
7. Adapt to ABI changes
8. Update expected test paths and addresses
9. Update docs

## Development - Predeployment

Several things are preconfigured on startup to be available on the first user interaction with Devnet. This is done in the `initialize` method of `StarknetWrapper`. The following is currently executed:

- Deployment of
  - Fee token contract
  - User accounts
  - Chargeable account
    - for e.g. signing minting txs
  - UDC
    - supports contract deployment ever since deploy txs have been deprecated
- Declaration of the account class used by Starknet CLI

## Development - Build

You don't need to build anything to be able to run locally, but if you need the `*.whl` or `*.tar.gz` artifacts, run

```text
$ poetry build
```

## Development - Version release

You can check the current version on master with these commands:

```text
$ git checkout master
$ poetry version
```

To update the version use:

```
$ poetry version <VERSION>
```

or any other variation of that [command](https://python-poetry.org/docs/cli/#version)

In `starknet_devnet/__init__.py` you need to manually update the version:

```
__version__ = "<VERSION>"
```

If you did everything correctly, these commands should result in the same version:

```
$ poetry version
$ poetry run starknet-devnet --version
```

Commit (Notice the `v`):

```
$ git add starknet_devnet/__init__.py pyproject.toml
$ git commit -m "Bump version to v<VERSION>"
$ git push
```

If the CI/CD pipeline ran successfully, tag the new version:

```
$ git tag v<VERSION>
$ git push origin v<VERSION>
```

Documentation is deployed automatically to https://shard-labs.github.io/starknet-devnet:

- Uses the FabijanC username and its private GitHub token set in the CircleCI project env vars.
- Done only after a new version is released.
- Initiated in `scripts/package_build_and_publish.sh`.

Lastly:

- Check if tests and version/image publishing ran successfully on CI
- Generate release notes with the corresponding tag version on GitHub
- Inform users on Telegram, [Discord Devnet channel](https://discord.com/channels/793094838509764618/985824027950055434), and [Starknet Shamans](https://community.starknet.io/t/starknet-devnet/69).
