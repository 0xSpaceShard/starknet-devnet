"""Test feeder gateway responses of origin and fork"""

import json

import requests
from starkware.starknet.definitions.error_codes import StarknetErrorCode
from starkware.starknet.services.api.feeder_gateway.response_objects import (
    BlockIdentifier,
)

from .account import declare, invoke
from .settings import APP_URL
from .shared import (
    BALANCE_KEY,
    CONTRACT_PATH,
    EXPECTED_CLASS_HASH,
    PREDEPLOYED_ACCOUNT_ADDRESS,
    PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
)
from .test_state_update import get_state_update
from .test_transaction_trace import (
    assert_get_block_traces_response,
    get_block_traces,
    get_transaction_trace_response,
)
from .testnet_deployment import (
    TESTNET_CONTRACT_ADDRESS,
    TESTNET_CONTRACT_CLASS_HASH,
    TESTNET_DEPLOYMENT_BLOCK,
    TESTNET_FORK_PARAMS,
    TESTNET_URL,
)
from .util import (
    assert_address_has_no_class_hash,
    assert_class_by_hash,
    assert_class_by_hash_not_present,
    assert_class_hash_at_address,
    assert_compiled_class_by_hash_not_present,
    assert_contract_code_not_present,
    assert_contract_code_present,
    assert_full_contract,
    assert_full_contract_not_present,
    assert_receipt_present,
    assert_storage,
    assert_transaction,
    assert_transaction_not_received,
    assert_transaction_receipt_not_received,
    assert_tx_status,
    deploy,
    devnet_in_background,
    get_block,
    get_full_contract,
)

DEPLOYMENT_INPUT = "10"
EXPECTED_DEPLOYMENT_ADDRESS = (
    "0x03f37e0679b8c295373f6452c5c68c2de5ee661b77e02af5fe1a416ce9be93f7"
)
EXPECTED_INVOKE_HASH = (
    "0x54dd317f451041c1d4f138538ee665bff505dbeeec575902122014b60d0ce06"
)


def _deploy_to_expected_address(contract=CONTRACT_PATH):
    deploy_info = deploy(
        contract=contract,
        inputs=[DEPLOYMENT_INPUT],
        salt="0x42",
    )
    assert int(deploy_info["address"], 16) == int(EXPECTED_DEPLOYMENT_ADDRESS, 16)


def _make_expected_invoke(gateway_url=APP_URL):
    invoke_tx_hash = invoke(
        calls=[(TESTNET_CONTRACT_ADDRESS, "increase_balance", [1, 2])],
        account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
        gateway_url=gateway_url,
    )
    assert int(invoke_tx_hash, 16) == int(EXPECTED_INVOKE_HASH, 16)


@devnet_in_background(
    *TESTNET_FORK_PARAMS,
    "--fork-block",
    # starting from an earlier block; otherwise the contract class is already present
    str(TESTNET_DEPLOYMENT_BLOCK - 1),
)
def test_contract_responses_before_deployment_on_origin():
    """Assert that get_full_contract etc. only makes sense on fork after deployment"""
    # full contract
    assert_full_contract_not_present(
        address=EXPECTED_DEPLOYMENT_ADDRESS, feeder_gateway_url=TESTNET_URL
    )
    assert_full_contract_not_present(
        address=EXPECTED_DEPLOYMENT_ADDRESS, feeder_gateway_url=APP_URL
    )

    # code
    assert_contract_code_not_present(
        address=EXPECTED_DEPLOYMENT_ADDRESS, feeder_gateway_url=TESTNET_URL
    )
    assert_contract_code_not_present(
        address=EXPECTED_DEPLOYMENT_ADDRESS, feeder_gateway_url=APP_URL
    )

    # class hash
    assert_address_has_no_class_hash(
        contract_address=EXPECTED_DEPLOYMENT_ADDRESS, feeder_gateway_url=TESTNET_URL
    )
    assert_address_has_no_class_hash(
        contract_address=EXPECTED_DEPLOYMENT_ADDRESS, feeder_gateway_url=APP_URL
    )

    # storage
    assert_storage(
        address=EXPECTED_DEPLOYMENT_ADDRESS,
        key=BALANCE_KEY,
        expected_value="0x0",
        feeder_gateway_url=TESTNET_URL,
    )
    assert_storage(
        address=EXPECTED_DEPLOYMENT_ADDRESS,
        key=BALANCE_KEY,
        expected_value="0x0",
        feeder_gateway_url=APP_URL,
    )

    _deploy_to_expected_address()

    # full contract
    assert_full_contract_not_present(
        address=EXPECTED_DEPLOYMENT_ADDRESS, feeder_gateway_url=TESTNET_URL
    )
    assert_full_contract(
        address=EXPECTED_DEPLOYMENT_ADDRESS,
        expected_path=CONTRACT_PATH,
        feeder_gateway_url=APP_URL,
    )

    # code
    assert_contract_code_not_present(
        address=EXPECTED_DEPLOYMENT_ADDRESS, feeder_gateway_url=TESTNET_URL
    )
    assert_contract_code_present(
        address=EXPECTED_DEPLOYMENT_ADDRESS, feeder_gateway_url=APP_URL
    )

    # class hash
    assert_address_has_no_class_hash(
        contract_address=EXPECTED_DEPLOYMENT_ADDRESS, feeder_gateway_url=TESTNET_URL
    )
    assert_class_hash_at_address(
        contract_address=EXPECTED_DEPLOYMENT_ADDRESS,
        expected_class_hash=EXPECTED_CLASS_HASH,
        feeder_gateway_url=APP_URL,
    )

    # storage
    assert_storage(
        address=EXPECTED_DEPLOYMENT_ADDRESS,
        key=BALANCE_KEY,
        expected_value="0x0",
        feeder_gateway_url=TESTNET_URL,
    )
    assert_storage(
        address=EXPECTED_DEPLOYMENT_ADDRESS,
        key=BALANCE_KEY,
        expected_value=hex(int(DEPLOYMENT_INPUT, 10)),
        feeder_gateway_url=APP_URL,
    )


@devnet_in_background(
    *TESTNET_FORK_PARAMS, "--fork-block", str(TESTNET_DEPLOYMENT_BLOCK)
)
def test_contract_responses_after_deployment_on_origin():
    """Assert get_full_contract etc. works for contract deployed earlier"""

    full_contract_testnet = get_full_contract(
        contract_address=TESTNET_CONTRACT_ADDRESS,
        feeder_gateway_url=TESTNET_URL,
    )
    full_contract_devnet = get_full_contract(
        contract_address=TESTNET_CONTRACT_ADDRESS,
        feeder_gateway_url=APP_URL,
    )
    assert full_contract_testnet == full_contract_devnet

    assert_contract_code_present(
        address=TESTNET_CONTRACT_ADDRESS, feeder_gateway_url=APP_URL
    )

    assert_class_hash_at_address(
        contract_address=TESTNET_CONTRACT_ADDRESS,
        expected_class_hash=TESTNET_CONTRACT_CLASS_HASH,
        feeder_gateway_url=APP_URL,
    )


@devnet_in_background(
    *TESTNET_FORK_PARAMS, "--fork-block", str(TESTNET_DEPLOYMENT_BLOCK - 1)
)
def test_declare_and_get_class_by_hash():
    """Test class declaration and class getting by hash"""

    assert_class_by_hash_not_present(
        class_hash=EXPECTED_CLASS_HASH, feeder_gateway_url=TESTNET_URL
    )
    assert_class_by_hash_not_present(
        class_hash=EXPECTED_CLASS_HASH, feeder_gateway_url=APP_URL
    )
    assert_compiled_class_by_hash_not_present(
        class_hash=EXPECTED_CLASS_HASH, feeder_gateway_url=TESTNET_URL
    )
    assert_compiled_class_by_hash_not_present(
        class_hash=EXPECTED_CLASS_HASH, feeder_gateway_url=APP_URL
    )

    declare_info = declare(
        contract_path=CONTRACT_PATH,
        account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
        max_fee=int(4e16),
    )
    assert int(declare_info["class_hash"], 16) == int(EXPECTED_CLASS_HASH, 16)
    assert_tx_status(declare_info["tx_hash"], "ACCEPTED_ON_L2")

    assert_class_by_hash_not_present(
        class_hash=EXPECTED_CLASS_HASH, feeder_gateway_url=TESTNET_URL
    )
    assert_class_by_hash(
        class_hash=EXPECTED_CLASS_HASH,
        expected_path=CONTRACT_PATH,
        feeder_gateway_url=APP_URL,
    )
    assert_compiled_class_by_hash_not_present(
        class_hash=EXPECTED_CLASS_HASH, feeder_gateway_url=TESTNET_URL
    )
    assert_compiled_class_by_hash_not_present(
        class_hash=EXPECTED_CLASS_HASH,
        feeder_gateway_url=APP_URL,
    )


def _assert_transaction_trace_not_present(tx_hash: str, feeder_gateway_url=APP_URL):
    resp = get_transaction_trace_response(tx_hash, server_url=feeder_gateway_url)
    assert resp.json()["code"] == str(StarknetErrorCode.INVALID_TRANSACTION_HASH)
    assert resp.status_code == 500


def _assert_transaction_trace_present(
    tx_hash: str, expected_address: str, feeder_gateway_url=APP_URL
):
    resp = get_transaction_trace_response(tx_hash, server_url=feeder_gateway_url)
    body = resp.json()
    assert body["function_invocation"]["contract_address"] == expected_address
    assert resp.status_code == 200


@devnet_in_background(
    *TESTNET_FORK_PARAMS, "--fork-block", str(TESTNET_DEPLOYMENT_BLOCK)
)
def test_transaction_responses():
    """Assert transaction only present on fork after invoking"""

    # tx status
    assert_tx_status(
        tx_hash=EXPECTED_INVOKE_HASH,
        expected_tx_status="NOT_RECEIVED",
        feeder_gateway_url=TESTNET_URL,
    )
    assert_tx_status(
        tx_hash=EXPECTED_INVOKE_HASH,
        expected_tx_status="NOT_RECEIVED",
        feeder_gateway_url=APP_URL,
    )

    # tx
    assert_transaction_not_received(
        tx_hash=EXPECTED_INVOKE_HASH, feeder_gateway_url=TESTNET_URL
    )
    assert_transaction_not_received(
        tx_hash=EXPECTED_INVOKE_HASH, feeder_gateway_url=APP_URL
    )

    # tx receipt
    assert_transaction_receipt_not_received(
        tx_hash=EXPECTED_INVOKE_HASH, feeder_gateway_url=TESTNET_URL
    )
    assert_transaction_receipt_not_received(
        tx_hash=EXPECTED_INVOKE_HASH, feeder_gateway_url=APP_URL
    )

    # tx trace
    _assert_transaction_trace_not_present(
        tx_hash=EXPECTED_INVOKE_HASH, feeder_gateway_url=TESTNET_URL
    )
    _assert_transaction_trace_not_present(
        tx_hash=EXPECTED_INVOKE_HASH, feeder_gateway_url=APP_URL
    )

    _make_expected_invoke()

    # tx status
    assert_tx_status(
        tx_hash=EXPECTED_INVOKE_HASH,
        expected_tx_status="NOT_RECEIVED",
        feeder_gateway_url=TESTNET_URL,
    )
    assert_tx_status(
        tx_hash=EXPECTED_INVOKE_HASH,
        expected_tx_status="ACCEPTED_ON_L2",
        feeder_gateway_url=APP_URL,
    )

    # tx
    assert_transaction_not_received(
        tx_hash=EXPECTED_INVOKE_HASH, feeder_gateway_url=TESTNET_URL
    )
    assert_transaction(
        tx_hash=EXPECTED_INVOKE_HASH,
        expected_status="ACCEPTED_ON_L2",
        feeder_gateway_url=APP_URL,
    )

    # tx receipt
    assert_transaction_receipt_not_received(
        tx_hash=EXPECTED_INVOKE_HASH, feeder_gateway_url=TESTNET_URL
    )
    assert_receipt_present(
        tx_hash=EXPECTED_INVOKE_HASH,
        expected_status="ACCEPTED_ON_L2",
        feeder_gateway_url=APP_URL,
    )

    # tx trace
    _assert_transaction_trace_not_present(
        tx_hash=EXPECTED_INVOKE_HASH, feeder_gateway_url=TESTNET_URL
    )
    _assert_transaction_trace_present(
        tx_hash=EXPECTED_INVOKE_HASH,
        expected_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        feeder_gateway_url=APP_URL,
    )


def _assert_block_artifact_not_found(
    method: str,
    block_number: BlockIdentifier = None,
    block_hash: str = None,
    feeder_gateway_url=APP_URL,
):
    resp = requests.get(
        f"{feeder_gateway_url}/feeder_gateway/{method}",
        {"blockNumber": block_number, "blockHash": block_hash},
    )
    assert json.loads(resp.text)["code"] == str(StarknetErrorCode.BLOCK_NOT_FOUND)
    assert resp.status_code == 500


@devnet_in_background(
    *TESTNET_FORK_PARAMS, "--fork-block", str(TESTNET_DEPLOYMENT_BLOCK)
)
def test_block_responses():
    """Test how block responses are handled when forking."""

    origin_block = get_block(
        block_number=TESTNET_DEPLOYMENT_BLOCK,
        parse=True,
        feeder_gateway_url=TESTNET_URL,
    )
    fork_block = get_block(block_number=TESTNET_DEPLOYMENT_BLOCK, parse=True)
    assert origin_block == fork_block

    # assert block count incremented by one (due to genesis block)
    latest_fork_block_before = get_block(block_number="latest", parse=True)
    assert latest_fork_block_before["block_number"] == fork_block["block_number"] + 1

    # assert next block not yet present
    next_block_number = str(TESTNET_DEPLOYMENT_BLOCK + 2)
    for method in "get_block", "get_block_traces", "get_state_update":
        _assert_block_artifact_not_found(method, block_number=next_block_number)

    # invoke
    _make_expected_invoke(gateway_url=APP_URL)

    # assert block count incremented by one
    latest_fork_block_after = get_block(block_number="latest", parse=True)
    assert (
        latest_fork_block_after["block_number"]
        == latest_fork_block_before["block_number"] + 1
    )

    # assert block trace
    assert_get_block_traces_response({"blockNumber": "latest"}, EXPECTED_INVOKE_HASH)

    # assert state update
    state_update = get_state_update(block_number="latest")
    assert TESTNET_CONTRACT_ADDRESS in state_update["state_diff"]["storage_diffs"]


@devnet_in_background(*TESTNET_FORK_PARAMS)
def test_block_responses_by_hash():
    """Test error is internally properly handled and that a json is sent"""
    dummy_hash = "0x1"
    for method in "get_block", "get_block_traces", "get_state_update":
        _assert_block_artifact_not_found(method, block_hash=dummy_hash)

    latest_block_by_number = get_block(block_number="latest", parse=True)
    latest_block_hash = latest_block_by_number["block_hash"]
    latest_block_by_hash = get_block(block_hash=latest_block_hash, parse=True)
    assert latest_block_by_number == latest_block_by_hash

    block_traces_by_hash = get_block_traces({"blockHash": latest_block_hash})
    block_traces_by_number = get_block_traces({"blockNumber": "latest"})
    assert block_traces_by_hash == block_traces_by_number

    state_update_by_hash = get_block_traces({"blockHash": latest_block_hash})
    state_update_by_number = get_block_traces({"blockNumber": "latest"})
    assert state_update_by_hash == state_update_by_number
