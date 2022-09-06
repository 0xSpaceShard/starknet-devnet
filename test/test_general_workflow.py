"""
Tests the general workflow of the devnet.
"""

import pytest

from .util import (
    assert_contract_class,
    assert_negative_block_input,
    assert_transaction_not_received,
    assert_transaction_receipt_not_received,
    assert_block,
    assert_contract_code,
    assert_equal,
    assert_failing_deploy,
    assert_receipt,
    assert_salty_deploy,
    assert_storage,
    assert_transaction,
    assert_tx_status,
    assert_events,
    call,
    deploy,
    get_class_by_hash,
    get_class_hash_at,
    get_full_contract,
    invoke,
)

from .shared import (
    ABI_PATH,
    BALANCE_KEY,
    CONTRACT_PATH,
    EVENTS_ABI_PATH,
    EVENTS_CONTRACT_PATH,
    EXPECTED_SALTY_DEPLOY_ADDRESS,
    EXPECTED_SALTY_DEPLOY_HASH,
    # EXPECTED_SALTY_DEPLOY_HASH_LITE_MODE,
    FAILING_CONTRACT_PATH,
    GENESIS_BLOCK_NUMBER,
    NONEXISTENT_TX_HASH,
)


@pytest.mark.usefixtures("run_devnet_in_background")
@pytest.mark.parametrize(
    "run_devnet_in_background, expected_hash",
    [
        ([], EXPECTED_SALTY_DEPLOY_HASH),
        # (["--lite-mode"], EXPECTED_SALTY_DEPLOY_HASH_LITE_MODE),
    ],
    indirect=True,
)
def test_general_workflow(expected_hash):
    """Test devnet with CLI"""
    deploy_info = deploy(CONTRACT_PATH, ["0"])

    assert_tx_status(deploy_info["tx_hash"], "ACCEPTED_ON_L2")
    assert_transaction(deploy_info["tx_hash"], "ACCEPTED_ON_L2")
    assert_transaction_not_received(NONEXISTENT_TX_HASH)

    # check storage after deployment
    assert_storage(deploy_info["address"], BALANCE_KEY, "0x0")

    # check block and receipt after deployment
    assert_negative_block_input()

    assert_block(GENESIS_BLOCK_NUMBER + 1, deploy_info["tx_hash"])
    assert_receipt(deploy_info["tx_hash"], "test/expected/deploy_receipt.json")
    assert_transaction_receipt_not_received(NONEXISTENT_TX_HASH)

    # check code
    assert_contract_code(deploy_info["address"])

    # check contract class
    class_by_address = get_full_contract(deploy_info["address"])
    assert_contract_class(class_by_address, CONTRACT_PATH)

    # check contract class through class hash
    class_hash = get_class_hash_at(deploy_info["address"])
    class_by_hash = get_class_by_hash(class_hash)
    assert_equal(class_by_address, class_by_hash)

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

    # check storage, block and receipt after increase
    assert_storage(deploy_info["address"], BALANCE_KEY, "0x1e")
    assert_block(GENESIS_BLOCK_NUMBER + 2, invoke_tx_hash)
    assert_receipt(invoke_tx_hash, "test/expected/invoke_receipt.json")

    # check handling complex input
    value = call(
        function="sum_point_array",
        address=deploy_info["address"],
        abi_path=ABI_PATH,
        inputs=["2", "10", "20", "30", "40"],
    )
    assert_equal(value, "40 60", "Checking complex input failed!")

    # check deploy when a salt is provided, and use the same contract to test events
    assert_salty_deploy(
        contract_path=EVENTS_CONTRACT_PATH,
        salt="0x99",
        inputs=None,
        expected_status="ACCEPTED_ON_L2",
        expected_address=EXPECTED_SALTY_DEPLOY_ADDRESS,
        expected_tx_hash=expected_hash,
    )

    assert_salty_deploy(
        contract_path=EVENTS_CONTRACT_PATH,
        salt="0x99",
        inputs=None,
        expected_status="ACCEPTED_ON_L2",
        expected_address=EXPECTED_SALTY_DEPLOY_ADDRESS,
        expected_tx_hash=expected_hash,
    )

    salty_invoke_tx_hash = invoke(
        function="increase_balance",
        address=EXPECTED_SALTY_DEPLOY_ADDRESS,
        abi_path=EVENTS_ABI_PATH,
        inputs=["10"],
    )

    assert_events(salty_invoke_tx_hash, "test/expected/invoke_receipt_event.json")

    assert_failing_deploy(contract_path=FAILING_CONTRACT_PATH)
