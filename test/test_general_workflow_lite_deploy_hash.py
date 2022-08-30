"""
The general workflow testing script, run with the lite deploy hash calculation mode.
"""

import pytest

from .util import (
    devnet_in_background,
    assert_equal,
    assert_tx_status,
    call,
    deploy,
    invoke,
)

from .shared import (
    ABI_PATH,
    CONTRACT_PATH,
)

BALANCE_KEY = (
    "916907772491729262376534102982219947830828984996257231353398618781993312401"
)


@pytest.mark.general_workflow
@devnet_in_background("--lite-mode-deploy-hash")
def test_general_workflow_lite():
    """Test devnet with CLI"""
    deploy_info = deploy(CONTRACT_PATH, ["0"])

    print("Deployment:", deploy_info)

    assert_tx_status(deploy_info["tx_hash"], "ACCEPTED_ON_L2")
    assert_equal(deploy_info["tx_hash"], "0x0")

    # increase and assert balance
    invoke(
        function="increase_balance",
        address=deploy_info["address"],
        abi_path=ABI_PATH,
        inputs=["10", "20"],
    )
    value = call(
        function="get_balance", address=deploy_info["address"], abi_path=ABI_PATH
    )
    assert_equal(value, "30", "Invoke+call failed!")
