"""Test cairo recompilers"""

import os
import subprocess

import pytest
from starkware.starknet.services.api.contract_class.contract_class import CompiledClass
from starkware.starknet.services.api.contract_class.contract_class_utils import (
    load_sierra,
)

from starknet_devnet.compiler import (
    ContractClassCompiler,
    CustomContractClassCompiler,
    DefaultContractClassCompiler,
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
from .util import (
    DevnetBackgroundProc,
    devnet_in_background,
    read_stream,
    terminate_and_wait,
)

SPECIFIED_MANIFEST = os.getenv("CAIRO_1_COMPILER_MANIFEST")
if not SPECIFIED_MANIFEST:
    raise KeyError("CAIRO_1_COMPILER_MANIFEST env var not set")

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
    [DefaultContractClassCompiler(), CustomContractClassCompiler(SPECIFIED_MANIFEST)],
)
def test_contract_class_compiler_happy_path(compiler: ContractClassCompiler):
    """Test the class abstracting the default compiler"""
    contract_class = load_sierra(CONTRACT_1_PATH)
    compiled = compiler.compile_contract_class(contract_class)

    with open(CONTRACT_1_CASM_PATH, encoding="utf-8") as casm_file:
        expected_compiled = CompiledClass.loads(casm_file.read())

    assert compiled == expected_compiled


@pytest.mark.parametrize("compiler_manifest", ["", "dummy-wrong"])
def test_invalid_cairo_compiler_manifest(compiler_manifest: str):
    """Test invalid cairo compiler manifest specified via CLI"""

    execution = ACTIVE_DEVNET.start(
        "--cairo-compiler-manifest",
        compiler_manifest,
        stderr=subprocess.PIPE,
        stdout=subprocess.PIPE,
    )

    assert execution.returncode != 0
    assert "Cairo compiler error" in read_stream(execution.stderr)
    assert read_stream(execution.stdout) == ""


def test_valid_cairo_compiler_manifest():
    """Test valid cairo compiler manifest specified via CLI"""
    execution = ACTIVE_DEVNET.start(
        "--cairo-compiler-manifest",
        SPECIFIED_MANIFEST,
        stderr=subprocess.PIPE,
        stdout=subprocess.PIPE,
    )
    terminate_and_wait(execution)
    assert execution.returncode == 0
    assert "Cairo compiler error" not in read_stream(execution.stderr)
    assert "Using cairo compiler" in read_stream(execution.stdout)


@devnet_in_background(
    *PREDEPLOY_ACCOUNT_CLI_ARGS, "--cairo-compiler-manifest", SPECIFIED_MANIFEST
)
def test_declaring_with_custom_compiler():
    """E2E test using cairo compiler specified via CLI"""
    contract_class, _, compiled_class_hash = load_cairo1_contract()
    resp = send_declare_v2(
        contract_class=contract_class,
        compiled_class_hash=compiled_class_hash,
        sender_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        sender_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
    )
    assert_declare_v2_accepted(resp)
