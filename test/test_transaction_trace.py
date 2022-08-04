"""
Test get_transaction endpoint
"""

import pytest
import requests
from starkware.starknet.services.api.feeder_gateway.response_objects import BlockTransactionTraces

from .util import declare, deploy, get_transaction_receipt, invoke, load_json_from_path, devnet_in_background
from .settings import APP_URL
from .shared import ABI_PATH, CONTRACT_PATH, SIGNATURE, NONEXISTENT_TX_HASH, GENESIS_BLOCK_NUMBER

def get_transaction_trace_response(tx_hash=None):
    """Get transaction trace response"""
    params = {
        "transactionHash": tx_hash,
    }

    res = requests.get(
        f"{APP_URL}/feeder_gateway/get_transaction_trace",
        params=params
    )

    return res

def deploy_empty_contract():
    """
    Deploy sample contract with balance = 0.
    Returns transaction hash.
    """
    return deploy(CONTRACT_PATH, inputs=["0"], salt="0x99")

def assert_function_invocation(function_invocation, expected_path):
    """Asserts function invocation"""
    expected_function_invocation = load_json_from_path(expected_path)
    assert function_invocation == expected_function_invocation

@pytest.mark.transaction_trace
@devnet_in_background()
def test_deploy_transaction_trace():
    """Test deploy transaction trace"""
    tx_hash = deploy_empty_contract()["tx_hash"]
    res = get_transaction_trace_response(tx_hash)

    assert res.status_code == 200

    transaction_trace = res.json()
    assert transaction_trace["signature"] == []
    assert_function_invocation(
        transaction_trace["function_invocation"],
        "test/expected/deploy_function_invocation.json"
    )

@pytest.mark.transaction_trace
@devnet_in_background()
def test_invoke_transaction_hash():
    """Test invoke transaction trace"""
    contract_address = deploy_empty_contract()["address"]
    tx_hash = invoke("increase_balance", ["10", "20"], contract_address, ABI_PATH)
    res = get_transaction_trace_response(tx_hash)

    assert res.status_code == 200

    transaction_trace = res.json()
    assert transaction_trace["signature"] == []
    assert_function_invocation(
        transaction_trace["function_invocation"],
        "test/expected/invoke_function_invocation.json"
    )


@pytest.mark.transaction_trace
@devnet_in_background()
def test_invoke_transaction_hash_with_signature():
    """Test invoke transaction trace with signature"""
    contract_address = deploy_empty_contract()["address"]
    tx_hash = invoke("increase_balance", ["10", "20"], contract_address, ABI_PATH, SIGNATURE)
    res = get_transaction_trace_response(tx_hash)

    assert res.status_code == 200

    transaction_trace = res.json()

    expected_signature = [hex(int(s)) for s in SIGNATURE]
    assert transaction_trace["signature"] == expected_signature

    assert_function_invocation(
        transaction_trace["function_invocation"],
        "test/expected/invoke_function_invocation.json"
    )

@pytest.mark.transaction_trace
@devnet_in_background()
def test_nonexistent_transaction_hash():
    """Test if it throws 500 for nonexistent transaction trace"""
    res = get_transaction_trace_response(NONEXISTENT_TX_HASH)

    assert res.status_code == 500

def assert_get_block_traces_response(params, expected_tx_hash):
    """Assert response of get_block_traces"""
    block_traces = requests.get(
        f"{APP_URL}/feeder_gateway/get_block_traces",
        params=params
    ).json()

    # loading to assert valid structure
    BlockTransactionTraces.load(block_traces)

    # index 0 assuming it's the only tx in the response
    actual_tx_hash = block_traces["traces"][0]["transaction_hash"]
    assert actual_tx_hash == expected_tx_hash

@pytest.mark.transaction_trace
@devnet_in_background()
def test_get_block_traces():
    """Test getting all traces of a block"""

    tx_hash = deploy_empty_contract()["tx_hash"]

    tx_receipt = get_transaction_receipt(tx_hash=tx_hash)
    block_hash = tx_receipt["block_hash"]

    assert_get_block_traces_response({ "blockHash": block_hash }, tx_hash)
    assert_get_block_traces_response({ "blockNumber": GENESIS_BLOCK_NUMBER + 1 }, tx_hash)
    assert_get_block_traces_response({}, tx_hash) # default behavior - no params provided

@pytest.mark.transaction_trace
@devnet_in_background()
def test_get_trace_and_block_traces_after_declare():
    """Test getting all traces of a block"""

    declare_dict = declare(CONTRACT_PATH)

    # assert trace
    trace_response = get_transaction_trace_response(declare_dict["tx_hash"])
    trace = trace_response.json()
    assert "function_invocation" in trace
    assert trace["signature"] == []

    assert_get_block_traces_response({}, declare_dict["tx_hash"])
