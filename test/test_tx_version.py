"""
Test transaction version
"""

import pytest

from starkware.starknet.definitions.constants import TRANSACTION_VERSION, QUERY_VERSION

from .util import devnet_in_background, deploy, call, invoke
from .shared import ARTIFACTS_PATH

CONTRACT_PATH = f"{ARTIFACTS_PATH}/tx_version.cairo/tx_version.json"
ABI_PATH = f"{ARTIFACTS_PATH}/tx_version.cairo/tx_version_abi.json"


@pytest.mark.tx_version
@devnet_in_background()
def test_transaction_version():
    """Check if transaction versions are correct"""
    deploy_info = deploy(CONTRACT_PATH)
    address = deploy_info["address"]

    invoke("set_tx_version", [], address, ABI_PATH)

    invoke_tx_version = call("get_last_tx_version", address, ABI_PATH)
    assert int(invoke_tx_version, 16) == TRANSACTION_VERSION

    call_tx_version = call("get_tx_version", address, ABI_PATH)

    assert int(call_tx_version, 16) == QUERY_VERSION
