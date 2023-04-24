"""Test custom account"""

import os
import subprocess

import pytest
from starkware.starknet.core.os.contract_class.deprecated_class_hash import (
    compute_deprecated_class_hash,
)
from starkware.starknet.services.api.contract_class.contract_class import (
    DeprecatedCompiledClass,
)

from .account import declare_and_deploy_with_chargeable, invoke
from .shared import (
    ABI_PATH,
    CONTRACT_PATH,
    PREDEPLOY_ACCOUNT_CLI_ARGS,
    PREDEPLOYED_ACCOUNT_ADDRESS,
    PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
)
from .util import (
    DevnetBackgroundProc,
    call,
    devnet_in_background,
    get_class_hash_at,
    load_file_content,
)

ACTIVE_DEVNET = DevnetBackgroundProc()

NON_EXISTENT_PATH = "most-certainly-non-existent-path.txt"
DIR_PATH = os.path.abspath(os.path.join(__file__, os.pardir))
MISSING_METHOD_PATH = os.path.join(
    __file__, os.pardir, "custom_account_missing_method.json"
)
CORRECT_PATH = os.path.join(__file__, os.pardir, "custom_account.json")


@pytest.mark.account_custom
@pytest.mark.parametrize(
    "class_path, expected_error",
    [
        (
            NON_EXISTENT_PATH,
            f"Error: {os.path.abspath(NON_EXISTENT_PATH)} is not a valid file\n",
        ),
        (DIR_PATH, f"Error: {DIR_PATH} is not a valid file\n"),
        (__file__, f"Error: {__file__} is not a valid JSON file\n"),
        (
            ABI_PATH,
            f"Error: {os.path.abspath(ABI_PATH)} is not a valid contract class artifact\n",
        ),
        (
            MISSING_METHOD_PATH,
            f"Error: {os.path.abspath(MISSING_METHOD_PATH)} is missing account methods: __validate_declare__\n",
        ),
    ],
)
def test_invalid_path(class_path: str, expected_error: str):
    """Test behavior on providing nonexistent path"""
    proc = ACTIVE_DEVNET.start("--account-class", class_path, stderr=subprocess.PIPE)
    assert proc.returncode == 1
    assert expected_error in proc.stderr.read().decode("utf-8")


@pytest.mark.account_custom
@devnet_in_background("--account-class", CORRECT_PATH, *PREDEPLOY_ACCOUNT_CLI_ARGS)
def test_providing_correct_account_class():
    """Test behavior if correct custom account provided"""
    fetched_class_hash = int(get_class_hash_at(PREDEPLOYED_ACCOUNT_ADDRESS), 16)

    expected_contract_class = DeprecatedCompiledClass.loads(
        load_file_content("custom_account.json")
    )
    assert fetched_class_hash == compute_deprecated_class_hash(expected_contract_class)

    deploy_info = declare_and_deploy_with_chargeable(CONTRACT_PATH, inputs=["0"])
    invoke(
        calls=[(deploy_info["address"], "increase_balance", [10, 20])],
        account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
    )
    increased_value = call(
        function="get_balance", address=deploy_info["address"], abi_path=ABI_PATH
    )

    assert increased_value == "30"
