"""Test cairo recompilers"""

import os
import subprocess
from typing import List

import pytest
from starkware.starknet.services.api.contract_class.contract_class import CompiledClass
from starkware.starknet.services.api.contract_class.contract_class_utils import (
    load_sierra,
)

from starknet_devnet.compiler import (
    BinaryContractClassCompiler,
    ContractClassCompiler,
    DefaultContractClassCompiler,
    ManifestContractClassCompiler,
)

from .account import send_declare_v2
from .shared import (
    CONTRACT_1_CASM_PATH,
    CONTRACT_1_PATH,
    PREDEPLOY_ACCOUNT_CLI_ARGS,
    PREDEPLOYED_ACCOUNT_ADDRESS,
    PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
)
from .test_declare_v2 import assert_declare_v2_accepted, load_cairo1_contract
from .util import DevnetBackgroundProc, read_stream, terminate_and_wait

CAIRO_1_COMPILER_MANIFEST = os.getenv("CAIRO_1_COMPILER_MANIFEST")
if not CAIRO_1_COMPILER_MANIFEST:
    raise KeyError("CAIRO_1_COMPILER_MANIFEST env var not set")

# since the manifest file is at the root of the compiler repo,
# this allows us to get the path of the repo itself
CAIRO_1_COMPILER_REPO = os.path.dirname(CAIRO_1_COMPILER_MANIFEST)

# assumes the artifacts were built in the repo with `cargo build --bin starknet-sierra-compile`
SIERRA_COMPILER_PATH = os.path.join(
    CAIRO_1_COMPILER_REPO, "target", "debug", "starknet-sierra-compile"
)

ACTIVE_DEVNET = DevnetBackgroundProc()


@pytest.fixture(autouse=True)
def run_before_and_after_test():
    """Cleanup after tests finish."""

    # before test
    ACTIVE_DEVNET.stop()
    yield
    # after test
    ACTIVE_DEVNET.stop()


@pytest.mark.parametrize(
    "compiler",
    [
        DefaultContractClassCompiler(),
        ManifestContractClassCompiler(CAIRO_1_COMPILER_MANIFEST),
        BinaryContractClassCompiler(SIERRA_COMPILER_PATH),
    ],
)
def test_contract_class_compiler_happy_path(compiler: ContractClassCompiler):
    """Test the class abstracting the default compiler"""
    contract_class = load_sierra(CONTRACT_1_PATH)
    compiled = compiler.compile_contract_class(contract_class)

    with open(CONTRACT_1_CASM_PATH, encoding="utf-8") as casm_file:
        expected_compiled = CompiledClass.loads(casm_file.read())

    assert compiled == expected_compiled


@pytest.mark.parametrize("manifest_value", ["", "dummy-wrong"])
def test_invalid_compiler_manifest(manifest_value: str):
    """Test invalid compiler manifest specified via CLI"""

    execution = ACTIVE_DEVNET.start(
        "--cairo-compiler-manifest",
        manifest_value,
        stderr=subprocess.PIPE,
        stdout=subprocess.PIPE,
    )

    assert execution.returncode != 0
    assert "Cairo compiler error" in read_stream(execution.stderr)
    assert read_stream(execution.stdout) == ""


@pytest.mark.parametrize("compiler_value", ["", "dummy-wrong"])
def test_invalid_sierra_compiler(compiler_value: str):
    """Test invalid sierra compiler specified via CLI"""

    execution = ACTIVE_DEVNET.start(
        "--sierra-compiler-path",
        compiler_value,
        stderr=subprocess.PIPE,
        stdout=subprocess.PIPE,
    )

    assert execution.returncode != 0
    assert (
        "The argument of --sierra-compiler-path must be an executable"
        in read_stream(execution.stderr)
    )
    assert read_stream(execution.stdout) == ""


@pytest.mark.parametrize(
    "cli_args",
    [
        ["--cairo-compiler-manifest", CAIRO_1_COMPILER_MANIFEST],
        ["--sierra-compiler-path", SIERRA_COMPILER_PATH],
    ],
)
def test_valid_compiler_specification(cli_args: List[str]):
    """Test valid cairo compiler specified via CLI"""
    execution = ACTIVE_DEVNET.start(
        *cli_args,
        stderr=subprocess.PIPE,
        stdout=subprocess.PIPE,
    )

    terminate_and_wait(execution)
    assert execution.returncode == 0

    stderr = read_stream(execution.stderr)
    assert "The argument of --sierra-compiler-path must be an executable" not in stderr
    assert "Cairo compiler error" not in stderr
    assert "Using cairo compiler" in read_stream(execution.stdout)


@pytest.mark.usefixtures("run_devnet_in_background")
@pytest.mark.parametrize(
    "run_devnet_in_background",
    [
        (
            *PREDEPLOY_ACCOUNT_CLI_ARGS,
            "--cairo-compiler-manifest",
            CAIRO_1_COMPILER_MANIFEST,
        ),
        (*PREDEPLOY_ACCOUNT_CLI_ARGS, "--sierra-compiler-path", SIERRA_COMPILER_PATH),
    ],
    indirect=True,
)
def test_declaring_with_custom_manifest():
    """E2E tests using compiler specified via CLI"""
    contract_class, _, compiled_class_hash = load_cairo1_contract()
    resp = send_declare_v2(
        contract_class=contract_class,
        compiled_class_hash=compiled_class_hash,
        sender_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        sender_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
    )
    assert_declare_v2_accepted(resp)


def test_manifest_and_sierra_compiler_specified():
    """Should fail if both modes specified"""
    execution = ACTIVE_DEVNET.start(
        "--cairo-compiler-manifest",
        CAIRO_1_COMPILER_MANIFEST,
        "--sierra-compiler-path",
        SIERRA_COMPILER_PATH,
        stderr=subprocess.PIPE,
        stdout=subprocess.PIPE,
    )

    terminate_and_wait(execution)
    assert execution.returncode != 0

    assert (
        "Only one of {--cairo-compiler-manifest,--sierra-compiler-path} can be provided"
        in read_stream(execution.stderr)
    )
