"""
Test transaction version
"""

import pytest
from starkware.starknet.definitions.constants import TRANSACTION_VERSION

from .account import invoke
from .shared import (
    ARTIFACTS_PATH,
    PREDEPLOY_ACCOUNT_CLI_ARGS,
    PREDEPLOYED_ACCOUNT_ADDRESS,
    PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
)
from .util import call, deploy, devnet_in_background

CONTRACT_PATH = f"{ARTIFACTS_PATH}/tx_version.cairo/tx_version.json"
ABI_PATH = f"{ARTIFACTS_PATH}/tx_version.cairo/tx_version_abi.json"


@pytest.mark.tx_version
@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS)
def test_transaction_version():
    """Check if transaction versions are correct"""
    deploy_info = deploy(CONTRACT_PATH)
    address = deploy_info["address"]

    invoke(
        calls=[(address, "set_tx_version", [])],
        account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
    )

    invoke_tx_version = call("get_last_tx_version", address, ABI_PATH)
    assert int(invoke_tx_version, 16) == TRANSACTION_VERSION

    call_tx_version = call("get_tx_version", address, ABI_PATH)
    # before starknet 0.10.0 it used to be QUERY_VERSION
    assert int(call_tx_version, 16) == TRANSACTION_VERSION
