---
sidebar_position: 18
---

# Development

If you're a developer willing to contribute, be sure to have installed [**Poetry**](https://pypi.org/project/poetry/) and all the dependency packages by running the installation script. Prerequisites for running the script: `gcc`, `g++`, `gmp`, `npm`.

```bash
./scripts/install_dev_tools.sh
```

## Development - Run

```text
poetry run starknet-devnet
```

## Development - Run in debug mode

This will restart Devnet on each code change:

```text
./scripts/starknet_devnet_debug.sh
```

## Development - Format and Lint

```text
./scripts/format.sh
./scripts/lint.sh
```

## Development - Test

When running tests locally, do it from the project root:

First generate the artifacts:

```bash
./scripts/compile_contracts.sh
```

Use one of the testing commands:

```bash
./scripts/test.sh [TEST_CASE] # parallelized testing - using auto detected number of CPU cores

poetry run pytest -s -v test/ # for more verbose output

poetry run pytest test/<TEST_FILE> # for a single file

poetry run pytest test/<TEST_FILE>::<TEST_CASE> # for a single test case
```

## Development - Check versioning consistency

```
./scripts/check_versions.sh
```

## Development - Working with an archive of cairo-lang

If you know the URL of the archive (e.g. ZIP) of a new cairo-lang version, you can install it with

```
poetry add <URL>
```

After adding a new cairo-lang version, you will probably want to recompile contract artifacts.

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

## Development - Build

You don't need to build anything to be able to run locally, but if you need the `*.whl` or `*.tar.gz` artifacts, run

```text
poetry build
```

## Development - Version release

You can check the current version on master with these commands:

```
git checkout master
poetry version
```

To update the version use:

```
poetry version <VERSION>
```

or any other variation of that [command](https://python-poetry.org/docs/cli/#version)

In `starknet_devnet/__init__.py` you need to manually update the version:

```
__version__ = "<VERSION>"
```

If you did everything correctly, these commands should result with the same version:

```
poetry version
poetry run starknet-devnet --version
```

Add a tag to the version update commit (Notice the `v`):

```
git tag v<VERSION>
git push origin v<VERSION>
```

Documentation is deployed automatically to https://shard-labs.github.io/starknet-devnet.

- This process uses the FabijanC username and its private GitHub token set in the CircleCI project env vars.
- This is done only after a new version is released.
- This is initiated in `scripts/package_build_and_publish.sh`.

Lastly:

- Check if tests and version/image publishing ran successfully on CI
- Generate release notes with the corresponding tag version on GitHub
- Inform users on Telegram, [Discord Devnet channel](https://discord.com/channels/793094838509764618/985824027950055434), and [StarkNet Shamans](https://community.starknet.io/t/starknet-devnet/69).
