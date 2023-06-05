"""
Tests the general workflow of the devnet.
"""

import pytest

from .account import declare_and_deploy_with_chargeable, deploy_with_chargeable, invoke
from .shared import (
    ABI_PATH,
    BALANCE_KEY,
    CONTRACT_PATH,
    EVENTS_CONTRACT_PATH,
    FAILING_CONTRACT_PATH,
    GENESIS_BLOCK_NUMBER,
    NONEXISTENT_TX_HASH,
    PREDEPLOY_ACCOUNT_CLI_ARGS,
    PREDEPLOYED_ACCOUNT_ADDRESS,
    PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
)
from .util import (
    assert_block,
    assert_class_by_hash,
    assert_contract_code_present,
    assert_equal,
    assert_full_contract,
    assert_hex_equal,
    assert_negative_block_input,
    assert_receipt,
    assert_storage,
    assert_transaction,
    assert_transaction_not_received,
    assert_transaction_receipt_not_received,
    assert_tx_status,
    call,
    devnet_in_background,
    get_block,
    get_class_hash_at,
)


@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS, "--lite-mode")
def test_lite_mode_block_hash():
    """Test lite mode"""
    deploy_info = declare_and_deploy_with_chargeable(CONTRACT_PATH, inputs=["0"])
    assert_tx_status(deploy_info["tx_hash"], "ACCEPTED_ON_L2")
    expected_block_hash = "0x2"  # after declare and deploy
    assert_equal(expected_block_hash, get_block(parse=True)["block_hash"])


@pytest.mark.usefixtures("run_devnet_in_background")
@pytest.mark.parametrize(
    "run_devnet_in_background",
    [
        PREDEPLOY_ACCOUNT_CLI_ARGS,
        [*PREDEPLOY_ACCOUNT_CLI_ARGS, "--lite-mode"],
    ],
    indirect=True,
)
@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS, "--lite-mode")
def test_general_workflow():
    """Test main feeder gateway calls"""
    deploy_info = declare_and_deploy_with_chargeable(CONTRACT_PATH, inputs=["0"])

    assert_tx_status(deploy_info["tx_hash"], "ACCEPTED_ON_L2")
    assert_transaction(deploy_info["tx_hash"], "ACCEPTED_ON_L2")
    assert_transaction_not_received(NONEXISTENT_TX_HASH)

    # check storage after deployment
    assert_storage(deploy_info["address"], BALANCE_KEY, "0x0")

    # check block and receipt after deployment
    assert_negative_block_input()

    assert_block(GENESIS_BLOCK_NUMBER + 2, deploy_info["tx_hash"])  # declare+deploy
    assert_transaction_receipt_not_received(NONEXISTENT_TX_HASH)

    # check code
    assert_contract_code_present(deploy_info["address"])

    # check contract class
    assert_full_contract(address=deploy_info["address"], expected_path=CONTRACT_PATH)

    # check contract class through class hash
    class_hash = get_class_hash_at(deploy_info["address"])
    assert_class_by_hash(class_hash, expected_path=CONTRACT_PATH)

    # increase and assert balance
    invoke_tx_hash = invoke(
        calls=[(deploy_info["address"], "increase_balance", [10, 20])],
        account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
    )
    assert_transaction(invoke_tx_hash, "ACCEPTED_ON_L2")
    assert_receipt(invoke_tx_hash, "ACCEPTED_ON_L2")
    value = call(
        function="get_balance", address=deploy_info["address"], abi_path=ABI_PATH
    )
    assert_equal(value, "30", "Invoke+call failed!")

    # check storage, block and receipt after increase
    assert_storage(deploy_info["address"], BALANCE_KEY, "0x1e")
    assert_block(GENESIS_BLOCK_NUMBER + 3, invoke_tx_hash)


@devnet_in_background()
def test_salty_deploy():
    """Test deploying with salt"""

    expected_address = (
        "0x947c548e04a9bd033e085960f736e5a39f398f726dcb378340d19a15f44a9b"
    )
    contract_path = EVENTS_CONTRACT_PATH
    inputs = None
    salt = "0x99"

    # first deploy (and declare before it)
    deploy_info = declare_and_deploy_with_chargeable(
        contract_path, inputs=inputs, salt=salt, max_fee=int(1e18)
    )
    assert_hex_equal(actual=deploy_info["address"], expected=expected_address)
    assert_tx_status(deploy_info["tx_hash"], "ACCEPTED_ON_L2")

    # attempt another deploy - should be rejected since address occupied
    repeated_deploy_info = deploy_with_chargeable(
        class_hash=deploy_info["class_hash"],
        inputs=inputs,
        salt=salt,
        # different max_fee to induce a different tx_hash
        max_fee=int(1e18) + 1,
    )
    assert_hex_equal(actual=repeated_deploy_info["address"], expected=expected_address)
    assert_tx_status(repeated_deploy_info["tx_hash"], "REJECTED")


@devnet_in_background()
def test_failing_deploy():
    """Test a failing deployment"""
    deploy_info = declare_and_deploy_with_chargeable(
        FAILING_CONTRACT_PATH,
        max_fee=int(1e18),  # if not provided, will fail on implicit estimation
    )
    assert_tx_status(deploy_info["tx_hash"], "REJECTED")
    assert_transaction(deploy_info["tx_hash"], "REJECTED")
