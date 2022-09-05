"""
Test block number
"""

import pytest

from .shared import ARTIFACTS_PATH, FAILING_CONTRACT_PATH, GENESIS_BLOCK_NUMBER
from .util import declare, devnet_in_background, deploy, call, invoke

BLOCK_NUMBER_CONTRACT_PATH = f"{ARTIFACTS_PATH}/block_number.cairo/block_number.json"
BLOCK_NUMBER_ABI_PATH = f"{ARTIFACTS_PATH}/block_number.cairo/block_number_abi.json"


def my_get_block_number(address: str):
    """Execute my_get_block_number on block_number.cairo contract deployed at `address`"""
    return call(
        function="my_get_block_number", address=address, abi_path=BLOCK_NUMBER_ABI_PATH
    )


@pytest.mark.usefixtures("run_devnet_in_background")
@pytest.mark.parametrize(
    "run_devnet_in_background, expected_hash",
    [
        ([], "0x4f1ea446f67c1be47619444eae4d8118f6e017d0e6fe16e89b3df03da38606d"),
        (["--lite-mode"], "0x0"),
    ],
    indirect=True,
)
def test_block_number_incremented(expected_hash):
    """
    Tests how block number is incremented in regular mode and lite mode.
    In regular mode with salt "0x42" our expected hash is
    0x4f1ea446f67c1be47619444eae4d8118f6e017d0e6fe16e89b3df03da38606d.
    In lite mode we expect 0x0 transaction hash.
    """

    deploy_info = deploy(BLOCK_NUMBER_CONTRACT_PATH, salt="0x42")
    block_number_before = my_get_block_number(deploy_info["address"])

    assert int(block_number_before) == GENESIS_BLOCK_NUMBER + 1
    assert expected_hash == deploy_info["tx_hash"]

    invoke(
        function="write_block_number",
        inputs=[],
        address=deploy_info["address"],
        abi_path=BLOCK_NUMBER_ABI_PATH,
    )

    written_block_number = call(
        function="read_block_number",
        inputs=[],
        address=deploy_info["address"],
        abi_path=BLOCK_NUMBER_ABI_PATH,
    )
    assert int(written_block_number) == GENESIS_BLOCK_NUMBER + 2

    block_number_after = my_get_block_number(deploy_info["address"])
    assert int(block_number_after) == GENESIS_BLOCK_NUMBER + 2


@devnet_in_background()
def test_block_number_incremented_on_declare():
    """Declare tx should increment get_block_number response"""

    deploy_info = deploy(BLOCK_NUMBER_CONTRACT_PATH)
    block_number_before = my_get_block_number(deploy_info["address"])
    assert int(block_number_before) == GENESIS_BLOCK_NUMBER + 1

    # just to declare a new class - nothing fails here
    declare(FAILING_CONTRACT_PATH)

    block_number_after = my_get_block_number(deploy_info["address"])
    assert int(block_number_after) == GENESIS_BLOCK_NUMBER + 2


@devnet_in_background()
def test_block_number_not_incremented_if_deploy_fails():
    """
    Since the deploy fails, no block should be created;
    get_block_number should return an unchanged value
    """

    deploy_info = deploy(BLOCK_NUMBER_CONTRACT_PATH)
    block_number_before = my_get_block_number(deploy_info["address"])
    assert int(block_number_before) == GENESIS_BLOCK_NUMBER + 1

    deploy(FAILING_CONTRACT_PATH)

    block_number_after = my_get_block_number(deploy_info["address"])
    assert int(block_number_after) == GENESIS_BLOCK_NUMBER + 1


@devnet_in_background()
def test_block_number_not_incremented_if_invoke_fails():
    """
    Since the invoke fails, no block should be created;
    get_block_number should return an unchanged value
    """

    deploy_info = deploy(BLOCK_NUMBER_CONTRACT_PATH)
    block_number_before = my_get_block_number(deploy_info["address"])
    assert int(block_number_before) == GENESIS_BLOCK_NUMBER + 1

    invoke(
        function="fail",
        inputs=[],
        address=deploy_info["address"],
        abi_path=BLOCK_NUMBER_ABI_PATH,
    )

    block_number_after = my_get_block_number(deploy_info["address"])
    assert int(block_number_after) == GENESIS_BLOCK_NUMBER + 1
