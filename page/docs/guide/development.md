---
sidebar_position: 16
---

# Development

If you're a developer willing to contribute, be sure to have installed [**Poetry**](https://pypi.org/project/poetry/) and all the dependency packages by running the following script. You are expected to have [**npm**](https://www.npmjs.com/).

```text
./scripts/install_dev_tools.sh
```

## Development - Run

```text
poetry run starknet-devnet
```

## Development - Run in debug mode

```text
./scripts/starknet_devnet_debug.sh
```

## Development - Lint

```text
./scripts/lint.sh
```

## Development - Test

When running tests locally, do it from the project root:

```bash
./scripts/compile_contracts.sh # first generate the artifacts

./scripts/test.sh [TEST_CASE] # parallelized testing - using auto detected number of CPU cores

poetry run pytest -s -v test/ # for more verbose output

poetry run pytest test/<TEST_FILE> # for a single file

poetry run pytest test/<TEST_FILE>::<TEST_CASE> # for a single test case
```

## Development - Check versioning consistency

```
./scripts/check_versions.sh
```

## Development - Working with a local version of cairo-lang

In `pyproject.toml` under `[tool.poetry.dependencies]` specify

```
cairo-lang = { path = "your-cairo-lang-package.zip" }
```

## Development - Updating accounts

1. Set up https://github.com/OpenZeppelin/cairo-contracts/ locally

   - `git clone ... && pip install cairo-nile && nile init`

2. `git checkout` to desired version
3. `nile compile --directory src`
4. Copy and minify `artifacts/Account.json` and `artifacts/abi/Account.json`
5. Update the precalculated hash

   - Predeployed account addresses should be intact

6. Update directory/file names containing the version
7. Update expected test paths and addresses
8. Update docs

## Development - Build

You don't need to build anything to be able to run locally, but if you need the `*.whl` or `*.tar.gz` artifacts, run

```text
poetry build
```