"""
The general workflow testing script, run with the lite block hash calculation mode.
"""

import pytest

from .util import (
    assert_block_hash,
    assert_negative_block_input,
    devnet_in_background,
    assert_block,
    assert_equal,
    assert_tx_status,
    call,
    deploy,
    invoke,
)

from .shared import ABI_PATH, CONTRACT_PATH, GENESIS_BLOCK_NUMBER

NONEXISTENT_TX_HASH = "0x12345678910111213"
BALANCE_KEY = (
    "916907772491729262376534102982219947830828984996257231353398618781993312401"
)


@pytest.mark.general_workflow
@devnet_in_background("--lite-mode-block-hash")
def test_general_workflow_lite():
    """Test devnet with CLI"""
    deploy_info = deploy(CONTRACT_PATH, ["0"])

    print("Deployment:", deploy_info)

    assert_tx_status(deploy_info["tx_hash"], "ACCEPTED_ON_L2")

    # check block and receipt after deployment
    assert_negative_block_input()

    assert_block(GENESIS_BLOCK_NUMBER + 1, deploy_info["tx_hash"])
    assert_block_hash(GENESIS_BLOCK_NUMBER + 1, hex(GENESIS_BLOCK_NUMBER + 1))

    # increase and assert balance
    invoke_tx_hash = invoke(
        function="increase_balance",
        address=deploy_info["address"],
        abi_path=ABI_PATH,
        inputs=["10", "20"],
    )
    value = call(
        function="get_balance", address=deploy_info["address"], abi_path=ABI_PATH
    )
    assert_equal(value, "30", "Invoke+call failed!")

    assert_block(GENESIS_BLOCK_NUMBER + 2, invoke_tx_hash)
    assert_block_hash(GENESIS_BLOCK_NUMBER + 2, hex(GENESIS_BLOCK_NUMBER + 2))
