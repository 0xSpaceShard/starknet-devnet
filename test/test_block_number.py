"""
Test block number
"""

from .shared import ARTIFACTS_PATH, GENESIS_BLOCK_NUMBER
from .util import create_empty_block, devnet_in_background, deploy, call

BLOCK_NUMBER_CONTRACT_PATH = f"{ARTIFACTS_PATH}/block_number.cairo/block_number.json"
BLOCK_NUMBER_ABI_PATH = f"{ARTIFACTS_PATH}/block_number.cairo/block_number_abi.json"

def my_get_block_number(address: str):
    """Execute my_get_block_number on block_number.cairo contract deployed at `address`"""
    return call(
        function="my_get_block_number",
        address=address,
        abi_path=BLOCK_NUMBER_ABI_PATH
    )

def base_workflow():
    """Used by test cases to perform the test"""
    deploy_info = deploy(BLOCK_NUMBER_CONTRACT_PATH)
    block_number_before = my_get_block_number(deploy_info["address"])
    assert int(block_number_before) == GENESIS_BLOCK_NUMBER + 1

    # generate a new block
    create_empty_block()

    block_number_after = my_get_block_number(deploy_info["address"])
    assert int(block_number_after) == GENESIS_BLOCK_NUMBER + 2

@devnet_in_background()
def test_block_number_incremented():
    """Tests how block number is incremented with"""
    base_workflow()

@devnet_in_background("--lite-mode")
def test_block_number__incremented_in_lite_mode():
    """Tests compatibility with lite mode"""
    base_workflow()
