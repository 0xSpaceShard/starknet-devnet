"""
Tests the abort block functionality.
"""
from test.rpc.rpc_utils import rpc_call

import requests
from starkware.starknet.definitions.error_codes import StarknetErrorCode

from starknet_devnet.blueprints.rpc.utils import rpc_felt

from .account import invoke
from .settings import APP_URL
from .shared import (
    CONTRACT_PATH,
    PREDEPLOY_ACCOUNT_CLI_ARGS,
    PREDEPLOYED_ACCOUNT_ADDRESS,
    PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
)
from .util import (
    assert_transaction,
    assert_tx_status,
    deploy,
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
    contract_deploy_info = deploy(CONTRACT_PATH, inputs=["0"])
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
    assert last_block["block_number"] == 0
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


@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS)
def test_abort_many_blocks_many_transactions():
    """Test abort of many blocks and many transactions."""

    # Block and transaction should be accepted on L2
    contract_deploy_info = deploy(CONTRACT_PATH, inputs=["0"])
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
        contract_deploy_block["block_hash"],
        invoke_block["block_hash"],
    ]
    last_block = get_block(parse=True)
    assert last_block["status"] == "ACCEPTED_ON_L2"
    assert last_block["block_number"] == 0
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

    # Block and transaction should be accepted on L2
    contract_deploy_info = deploy(CONTRACT_PATH, inputs=["0"])
    last_block = get_block(parse=True)
    assert last_block["status"] == "ACCEPTED_ON_L2"
    assert last_block["block_number"] == 1
    last_block_by_number = get_block(block_number=1, parse=True)
    assert last_block_by_number["block_number"] == last_block["block_number"]
    assert last_block_by_number["block_hash"] == last_block["block_hash"]
    assert_tx_status(contract_deploy_info["tx_hash"], "ACCEPTED_ON_L2")
