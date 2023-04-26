"""
Tests the abort block functionality.
"""
from test.rpc.rpc_utils import rpc_call

import requests
from starkware.starknet.definitions.error_codes import StarknetErrorCode

from starknet_devnet.blueprints.rpc.utils import rpc_felt

from .account import declare_and_deploy_with_chargeable, invoke
from .settings import APP_URL
from .shared import (
    ABI_PATH,
    CONTRACT_PATH,
    PREDEPLOY_ACCOUNT_CLI_ARGS,
    PREDEPLOYED_ACCOUNT_ADDRESS,
    PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
)
from .testnet_deployment import TESTNET_DEPLOYMENT_BLOCK, TESTNET_FORK_PARAMS
from .util import (
    assert_transaction,
    assert_tx_status,
    call,
    demand_block_creation,
    devnet_in_background,
    get_block,
)

NON_EXISTENT_BLOCK = "0x9"


def abort_blocks(block_hash):
    """Abort blocks after certain block hash"""
    return requests.post(
        f"{APP_URL}/abort_blocks", json={"startingBlockHash": block_hash}
    )


@devnet_in_background()
def test_abort_not_existing_block():
    """Test abort of not existing block."""
    response = abort_blocks(NON_EXISTENT_BLOCK)
    assert response.json()["code"] == str(StarknetErrorCode.BLOCK_NOT_FOUND)
    assert response.status_code == 500


@devnet_in_background()
def test_abort_single_block_single_transaction():
    """Test abort of single block and single transaction."""

    # Block and transaction should be accepted on L2
    contract_deploy_info = declare_and_deploy_with_chargeable(
        contract=CONTRACT_PATH, inputs=[0]
    )
    contract_deploy_block = get_block(parse=True)
    assert contract_deploy_block["status"] == "ACCEPTED_ON_L2"
    assert_tx_status(contract_deploy_info["tx_hash"], "ACCEPTED_ON_L2")

    # Blocks should be aborted and transactions should be rejected
    response = abort_blocks(contract_deploy_block["block_hash"])
    assert response.status_code == 200
    assert response.json()["aborted"] == [
        contract_deploy_block["block_hash"],
    ]
    last_block = get_block(parse=True)
    assert last_block["status"] == "ACCEPTED_ON_L2"
    assert last_block["block_number"] == 1
    contract_deploy_block_after_abort = get_block(
        block_hash=contract_deploy_block["block_hash"], parse=True
    )
    assert contract_deploy_block_after_abort["status"] == "ABORTED"
    assert_transaction(contract_deploy_info["tx_hash"], "REJECTED")

    # Test RPC get block status mapping from ABORTED to REJECTED
    rpc_aborted_block = rpc_call(
        "starknet_getBlockWithTxs",
        params={
            "block_id": {"block_hash": rpc_felt(contract_deploy_block["block_hash"])}
        },
    )
    assert rpc_aborted_block["result"]["status"] == "REJECTED"


@devnet_in_background()
def test_abort_same_block_twice():
    """Test abort of the same block twice."""

    # Block and transaction should be accepted on L2
    contract_deploy_info = declare_and_deploy_with_chargeable(
        contract=CONTRACT_PATH, inputs=[0]
    )
    contract_deploy_block = get_block(parse=True)
    assert contract_deploy_block["status"] == "ACCEPTED_ON_L2"
    assert_tx_status(contract_deploy_info["tx_hash"], "ACCEPTED_ON_L2")

    # Blocks should be aborted and transactions should be rejected
    response = abort_blocks(contract_deploy_block["block_hash"])
    assert response.status_code == 200
    assert response.json()["aborted"] == [
        contract_deploy_block["block_hash"],
    ]
    contract_deploy_block_after_abort = get_block(
        block_hash=contract_deploy_block["block_hash"], parse=True
    )
    assert contract_deploy_block_after_abort["status"] == "ABORTED"
    assert_transaction(contract_deploy_info["tx_hash"], "REJECTED")

    # Try to abort block again
    response = abort_blocks(contract_deploy_block["block_hash"])
    assert response.status_code == 400
    assert (
        response.json()["message"]
        == "Block cannot be aborted. Make sure you are aborting an accepted block."
    )


@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS)
def test_abort_many_blocks_many_transactions():
    """Test abort of many blocks and many transactions."""

    # Block and transaction should be accepted on L2
    contract_deploy_info = declare_and_deploy_with_chargeable(
        contract=CONTRACT_PATH, inputs=[0]
    )
    contract_deploy_block = get_block(parse=True)
    assert contract_deploy_block["status"] == "ACCEPTED_ON_L2"
    assert_tx_status(contract_deploy_info["tx_hash"], "ACCEPTED_ON_L2")

    # Block and transaction should be accepted on L2
    invoke_tx_hash = invoke(
        calls=[(contract_deploy_info["address"], "increase_balance", [10, 20])],
        account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
    )
    invoke_block = get_block(parse=True)
    assert invoke_block["status"] == "ACCEPTED_ON_L2"
    assert_transaction(invoke_tx_hash, "ACCEPTED_ON_L2")

    # Blocks should be aborted and transactions should be rejected
    response = abort_blocks(contract_deploy_block["block_hash"])
    assert response.status_code == 200
    assert response.json()["aborted"] == [
        invoke_block["block_hash"],
        contract_deploy_block["block_hash"],
    ]
    last_block = get_block(parse=True)
    assert last_block["status"] == "ACCEPTED_ON_L2"
    assert last_block["block_number"] == 1

    contract_deploy_block_after_abort = get_block(
        block_hash=contract_deploy_block["block_hash"], parse=True
    )
    assert contract_deploy_block_after_abort["status"] == "ABORTED"
    assert_transaction(contract_deploy_info["tx_hash"], "REJECTED")

    invoke_block_after_abort = get_block(
        block_hash=invoke_block["block_hash"], parse=True
    )
    assert invoke_block_after_abort["status"] == "ABORTED"
    assert_transaction(invoke_tx_hash, "REJECTED")

    # Test RPC get block status mapping from ABORTED to REJECTED
    rpc_aborted_block_contract = rpc_call(
        "starknet_getBlockWithTxs",
        params={
            "block_id": {"block_hash": rpc_felt(contract_deploy_block["block_hash"])}
        },
    )
    assert rpc_aborted_block_contract["result"]["status"] == "REJECTED"
    rpc_aborted_block_invoke = rpc_call(
        "starknet_getBlockWithTxs",
        params={"block_id": {"block_hash": rpc_felt(invoke_block["block_hash"])}},
    )
    assert rpc_aborted_block_invoke["result"]["status"] == "REJECTED"


@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS)
def test_new_blocks_after_abortion():
    """Test new block generation after abortion."""

    # Create and abort new block
    contract_deploy_info = declare_and_deploy_with_chargeable(
        contract=CONTRACT_PATH, inputs=[0]
    )
    contract_declare_block = get_block(block_number=1, parse=True)
    abort_blocks(contract_declare_block["block_hash"])

    # New block and transaction should be accepted on L2
    contract_deploy_info = declare_and_deploy_with_chargeable(
        contract=CONTRACT_PATH, inputs=[0]
    )
    last_block = get_block(parse=True)
    assert last_block["status"] == "ACCEPTED_ON_L2"
    assert last_block["block_number"] == 2
    last_block_by_number = get_block(block_number=2, parse=True)
    assert last_block_by_number["block_number"] == last_block["block_number"]
    assert last_block_by_number["block_hash"] == last_block["block_hash"]
    assert_tx_status(contract_deploy_info["tx_hash"], "ACCEPTED_ON_L2")


@devnet_in_background(
    *TESTNET_FORK_PARAMS, "--fork-block", str(TESTNET_DEPLOYMENT_BLOCK)
)
def test_forked_at_block_with_abort_blocks():
    """Test if abortion of forked blocks is failing."""
    # Get fork status
    fork_status = requests.get(f"{APP_URL}/fork_status")
    assert fork_status.status_code == 200

    # Abort block should fail on forked blocks
    block_before_fork = get_block(
        block_number=fork_status.json().get("block") - 1, parse=True
    )
    response = abort_blocks(block_before_fork["block_hash"])
    assert response.status_code == 500
    assert response.json()["message"] == "Aborting forked blocks is not supported."

    # Abort block should fail on genesis forked blocks
    genesis_block_fork = get_block(
        block_number=fork_status.json().get("block") + 1, parse=True
    )
    response = abort_blocks(genesis_block_fork["block_hash"])
    assert response.status_code == 500
    assert response.json()["message"] == "Aborting genesis block is not supported."

    # Deploy contract and mine new block
    contract_deploy_info = declare_and_deploy_with_chargeable(
        contract=CONTRACT_PATH, inputs=[0]
    )
    assert_tx_status(contract_deploy_info["tx_hash"], "ACCEPTED_ON_L2")

    # Abort Block should succeed on new block
    latest_block = get_block(parse=True)
    response = abort_blocks(latest_block["block_hash"])
    assert response.status_code == 200
    assert response.json()["aborted"] == [
        latest_block["block_hash"],
    ]


@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS)
def test_state_revert_with_abort_block():
    """Test state revert with aborted block."""

    # Block and transaction should be accepted on L2
    contract_deploy_info = declare_and_deploy_with_chargeable(
        contract=CONTRACT_PATH, inputs=[0]
    )
    contract_deploy_block = get_block(parse=True)
    assert contract_deploy_block["status"] == "ACCEPTED_ON_L2"
    assert_tx_status(contract_deploy_info["tx_hash"], "ACCEPTED_ON_L2")
    value_after_deploy = call(
        function="get_balance",
        address=contract_deploy_info["address"],
        abi_path=ABI_PATH,
    )
    assert int(value_after_deploy) == 0

    # Block and transaction should be accepted on L2
    invoke_tx_hash = invoke(
        calls=[(contract_deploy_info["address"], "increase_balance", [10, 20])],
        account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
    )
    invoke_block = get_block(parse=True)
    assert invoke_block["status"] == "ACCEPTED_ON_L2"
    assert_transaction(invoke_tx_hash, "ACCEPTED_ON_L2")
    value_after_invoke = call(
        function="get_balance",
        address=contract_deploy_info["address"],
        abi_path=ABI_PATH,
    )
    assert int(value_after_invoke) == 30

    # Block should be aborted and transaction should be rejected
    response = abort_blocks(invoke_block["block_hash"])
    assert response.status_code == 200
    assert response.json()["aborted"] == [invoke_block["block_hash"]]

    # Balance should be 0
    value_after_abort = call(
        function="get_balance",
        address=contract_deploy_info["address"],
        abi_path=ABI_PATH,
    )
    assert int(value_after_abort) == 0


@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS)
def test_abort_genesis_block():
    """Test abort of genesis block that should fail."""
    genesis_block = get_block(parse=True)
    response = abort_blocks(genesis_block["block_hash"])
    assert response.status_code == 500
    assert response.json()["message"] == "Aborting genesis block is not supported."


@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS, "--blocks-on-demand")
def test_pending_state_with_abort_block():
    """Test pending state with abort_block."""
    # Deploy the contract and create a block
    contract_deploy_info = declare_and_deploy_with_chargeable(
        contract=CONTRACT_PATH, inputs=[0]
    )
    demand_block_creation()

    # Transaction should be pending
    invoke_tx_hash = invoke(
        calls=[(contract_deploy_info["address"], "increase_balance", [10, 20])],
        account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
    )
    assert_tx_status(invoke_tx_hash, "PENDING")

    # Block should be aborted and pending transaction should be rejected
    latest_block = get_block(block_number="latest", parse=True)
    response = abort_blocks(latest_block["block_hash"])
    assert response.status_code == 200
    assert_tx_status(invoke_tx_hash, "REJECTED")
