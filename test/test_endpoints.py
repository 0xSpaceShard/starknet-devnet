"""
Test endpoints directly.
"""

import json

import pytest
import requests
from starkware.starknet.definitions.error_codes import StarknetErrorCode
from starkware.starkware_utils.error_handling import StarkErrorCode

from starknet_devnet.constants import DEFAULT_GAS_PRICE
from starknet_devnet.server import app

from .account import declare_and_deploy_with_chargeable
from .settings import APP_URL
from .shared import (
    FAILING_CONTRACT_PATH,
    GENESIS_BLOCK_HASH,
    GENESIS_BLOCK_NUMBER,
    STORAGE_CONTRACT_PATH,
)
from .support.assertions import assert_valid_schema
from .util import create_empty_block, devnet_in_background, load_file_content

INVOKE_CONTENT = load_file_content("invoke.json")
CALL_CONTENT = load_file_content("call.json")
INVALID_HASH = "0x58d4d4ed7580a7a98ab608883ec9fe722424ce52c19f2f369eeea301f535914"
INVALID_ADDRESS = "0x123"
INVALID_TRANSACTION_HASH_MESSAGE_PREFIX = (
    "Transaction hash should be a hexadecimal string starting with 0x, or 'null';"
)


def send_transaction(req_dict: dict):
    """Sends the dict in a POST request and returns the response data."""
    return app.test_client().post(
        "/gateway/add_transaction",
        content_type="application/json",
        data=json.dumps(req_dict),
    )


def send_call(req_dict: dict):
    """Sends the call dict in a POST request and returns the response data."""
    return app.test_client().post(
        "/feeder_gateway/call_contract",
        content_type="application/json",
        data=json.dumps(req_dict),
    )


def assert_invoke_resp(resp: bytes):
    """Asserts the validity of invoke response body."""
    resp_dict = json.loads(resp.data.decode("utf-8"))
    assert set(resp_dict.keys()) == set(["address", "code", "transaction_hash"])
    assert resp_dict["code"] == "TRANSACTION_RECEIVED"


@pytest.mark.deploy
def test_rejection_of_deprecated_deploy():
    """Deprecated deploy should be rejected"""
    resp = app.test_client().post(
        "/gateway/add_transaction",
        content_type="application/json",
        data=load_file_content("deprecated_deploy.json"),
    )
    assert resp.status_code == 500, resp.json
    assert resp.json == {
        "code": str(StarknetErrorCode.DEPRECATED_TRANSACTION),
        "message": "Deploy transaction is no longer supported.",
    }


@pytest.mark.invoke
def test_invoke_without_signature():
    """Invoke without signature"""
    req_dict = json.loads(INVOKE_CONTENT)
    del req_dict["signature"]
    resp = send_transaction(req_dict)
    assert resp.status_code == 400


@pytest.mark.invoke
def test_invoke_without_calldata():
    """Invoke without calldata"""
    req_dict = json.loads(INVOKE_CONTENT)
    del req_dict["calldata"]
    resp = send_transaction(req_dict)
    assert resp.status_code == 400


@pytest.mark.call
def test_call_with_invalid_signature():
    """Call without signature"""
    req_dict = json.loads(CALL_CONTENT)
    req_dict["signature"] = ["invalid_signature_obviously"]
    resp = send_call(req_dict)
    assert resp.status_code == 400


@pytest.mark.call
def test_call_without_calldata():
    """Call without calldata"""
    req_dict = json.loads(CALL_CONTENT)
    del req_dict["calldata"]
    resp = send_call(req_dict)
    assert resp.status_code == 400


# Error response tests
def send_transaction_with_requests(req_dict: dict):
    """Sends the dict in a POST request and returns the response data."""
    return requests.post(
        f"{APP_URL}/gateway/add_transaction", json=json.dumps(req_dict)
    )


def send_call_with_requests(req_dict: dict):
    """Sends the call dict in a POST request and returns the response data."""
    return requests.post(
        f"{APP_URL}/feeder_gateway/call_contract", json=json.dumps(req_dict)
    )


def get_block_by_number(req_dict: dict):
    """Get block number from request dict"""
    block_number = req_dict["blockNumber"]
    return requests.get(
        f"{APP_URL}/feeder_gateway/get_block?blockNumber={block_number}"
    )


def get_block_by_hash(block_hash: str):
    """Get block by block hash"""
    return requests.get(f"{APP_URL}/feeder_gateway/get_block?blockHash={block_hash}")


def get_transaction_trace(transaction_hash: str):
    """Get transaction trace from request dict"""
    return requests.get(
        f"{APP_URL}/feeder_gateway/get_transaction_trace?transactionHash={transaction_hash}"
    )


def get_transaction_trace_test_client(tx_hash: str):
    """Get transaction trace from request dict"""
    return app.test_client().get(
        f"{APP_URL}/feeder_gateway/get_transaction_trace?transactionHash={tx_hash}"
    )


def get_transaction_test_client(tx_hash: str):
    """Get transaction from request tx_hash"""
    return app.test_client().get(
        f"{APP_URL}/feeder_gateway/get_transaction?transactionHash={tx_hash}"
    )


def get_full_contract(contract_adress):
    """Get full contract class of a contract at a specific address"""
    return requests.get(
        f"{APP_URL}/feeder_gateway/get_full_contract?contractAddress={contract_adress}"
    )


def get_class_by_hash(class_hash: str):
    """Get contract class by class hash"""
    return requests.get(
        f"{APP_URL}/feeder_gateway/get_class_by_hash?classHash={class_hash}"
    )


def get_class_hash_at(contract_address: str):
    """Get class hash of a contract at the provided address"""
    return requests.get(
        f"{APP_URL}/feeder_gateway/get_class_hash_at?contractAddress={contract_address}"
    )


def get_state_update(block_hash, block_number):
    """Get state update"""
    return requests.get(
        f"{APP_URL}/feeder_gateway/get_state_update?blockHash={block_hash}&blockNumber={block_number}"
    )


def get_transaction_status(tx_hash):
    """Get transaction status"""
    response = requests.get(
        f"{APP_URL}/feeder_gateway/get_transaction_status?transactionHash={tx_hash}"
    )
    assert response.status_code == 200
    return response.json()


def get_transaction_status_test_client(tx_hash: str):
    """Get transaction status"""
    return app.test_client().get(
        f"{APP_URL}/feeder_gateway/get_transaction_status?transactionHash={tx_hash}"
    )


def get_transaction_receipt_test_client(tx_hash: str):
    """Get transaction receipt"""
    return app.test_client().get(
        f"{APP_URL}/feeder_gateway/get_transaction_receipt?transactionHash={tx_hash}"
    )


@pytest.mark.call
@devnet_in_background()
def test_error_response_call_without_calldata():
    """Call without calldata"""
    req_dict = json.loads(CALL_CONTENT)
    del req_dict["calldata"]
    resp = send_call_with_requests(req_dict)

    json_error_message = resp.json()["message"]
    assert resp.status_code == 400
    assert json_error_message is not None


@pytest.mark.call
@devnet_in_background()
def test_error_response_call_with_negative_block_number():
    """Call with negative block number"""
    resp = get_block_by_number({"blockNumber": -1})

    json_error_message = resp.json()["message"]
    assert resp.status_code == 500
    assert json_error_message is not None


@pytest.mark.call
@devnet_in_background()
def test_error_response_call_with_block_hash_0():
    """Should fail on call with block hash 0 without 0x prefix"""
    resp = get_block_by_hash("0")

    json_error_message = resp.json()["message"]
    assert resp.status_code == 500
    assert json_error_message.startswith(
        "Block hash should be a hexadecimal string starting with 0x, or 'null';"
    )


@pytest.mark.call
@devnet_in_background()
def test_error_response_call_with_invalid_transaction_hash():
    """Call with invalid transaction hash"""
    resp = get_transaction_trace(INVALID_HASH)

    json_error_message = resp.json()["message"]
    msg = "Transaction corresponding to hash"
    assert resp.status_code == 500
    assert json_error_message.startswith(msg)


@pytest.mark.call
@devnet_in_background()
def test_error_response_call_with_unavailable_contract():
    """Call with unavailable contract"""
    resp = get_full_contract(INVALID_HASH)

    json_error_message = resp.json()["message"]
    assert resp.status_code == 500
    assert json_error_message is not None


@pytest.mark.call
@devnet_in_background()
def test_error_response_call_with_state_update():
    """Call with unavailable state update"""
    resp = get_state_update(INVALID_HASH, -1)

    json_error_message = resp.json()["message"]
    assert resp.status_code == 500
    assert json_error_message is not None


@devnet_in_background()
def test_error_response_class_hash_at():
    """Get class hash of invalid address"""

    resp = get_class_hash_at(INVALID_ADDRESS)
    error_message = resp.json()["message"]

    assert resp.status_code == 500
    expected_message = (
        # alpha-goerli reports a decimal address
        f"Contract with address {int(INVALID_ADDRESS, 16)} is not deployed."
    )
    assert expected_message == error_message


@devnet_in_background()
def test_error_response_class_by_hash():
    """Get class by invalid hash"""

    resp = get_class_by_hash(INVALID_HASH)
    error_message = resp.json()["message"]

    assert resp.status_code == 500
    expected_message = f"Class with hash {INVALID_HASH} is not declared."
    assert expected_message == error_message


@devnet_in_background()
def test_create_block_endpoint():
    """Test empty block creation"""
    resp = get_block_by_number({"blockNumber": "latest"}).json()
    assert resp.get("block_hash") == GENESIS_BLOCK_HASH
    assert resp.get("block_number") == GENESIS_BLOCK_NUMBER

    create_empty_block()
    resp = get_block_by_number({"blockNumber": "latest"}).json()
    assert resp.get("block_number") == GENESIS_BLOCK_NUMBER + 1
    assert resp.get("block_hash") == hex(GENESIS_BLOCK_NUMBER + 1)
    assert resp.get("status") == "ACCEPTED_ON_L2"
    assert resp.get("gas_price") == hex(DEFAULT_GAS_PRICE)
    assert resp.get("transactions") == []

    declare_and_deploy_with_chargeable(STORAGE_CONTRACT_PATH)
    resp = get_block_by_number({"blockNumber": "latest"}).json()
    assert resp.get("block_number") == GENESIS_BLOCK_NUMBER + 3

    create_empty_block()
    resp = get_block_by_number({"blockNumber": "latest"}).json()
    assert resp.get("block_number") == GENESIS_BLOCK_NUMBER + 4
    assert resp.get("block_hash") == hex(GENESIS_BLOCK_NUMBER + 4)


@devnet_in_background()
def test_get_transaction_status():
    """Assert valid response schema"""
    # Create Transaction
    response = requests.post(
        f"{APP_URL}/mint",
        json={
            "address": "0x0513493b4Fe460031d445fFACacACf3B19196a05Fd146Ed1609B7248101eF847",
            "amount": 1000e18,
        },
    )
    assert response.status_code == 200
    tx_hash = response.json().get("tx_hash")

    json_response = get_transaction_status(tx_hash)
    assert_valid_schema(json_response, "get_transaction_status.json")
    assert json_response.get("tx_status") == "ACCEPTED_ON_L2"

    invalid_tx_hash = "0x443a8b3ec1f9e0c64"
    json_response = get_transaction_status(invalid_tx_hash)
    assert_valid_schema(json_response, "get_transaction_status.json")
    assert json_response.get("tx_status") == "NOT_RECEIVED"


@devnet_in_background()
def test_get_transaction_trace_of_rejected():
    """Send a failing tx and assert its trace"""
    deploy_info = declare_and_deploy_with_chargeable(
        contract=FAILING_CONTRACT_PATH, max_fee=int(1e18)
    )
    resp = get_transaction_trace(deploy_info["tx_hash"])
    resp_body = resp.json()
    assert resp_body["code"] == str(StarknetErrorCode.NO_TRACE)
    assert resp.status_code == 500


@pytest.mark.parametrize("tx_hash", ["0xyz", "0"])
def test_get_transaction_with_tx_hash(tx_hash):
    """Should fail on get_transaction with invalid hash"""
    resp = get_transaction_test_client(tx_hash)
    assert resp.json["message"].startswith(INVALID_TRANSACTION_HASH_MESSAGE_PREFIX)
    assert resp.status_code == 500


def test_get_transaction_status_with_tx_hash_0():
    """Should fail on get_transaction_status with hash 0 without 0x prefix"""
    resp = get_transaction_status_test_client("0")
    assert resp.json["message"].startswith(INVALID_TRANSACTION_HASH_MESSAGE_PREFIX)
    assert resp.status_code == 500


def test_get_transaction_trace_with_tx_hash_0():
    """Should fail on get_transaction_trace with hash 0 without 0x prefix"""
    resp = get_transaction_trace_test_client("0")
    assert resp.json["message"].startswith(INVALID_TRANSACTION_HASH_MESSAGE_PREFIX)
    assert resp.status_code == 500


def test_get_transaction_receipt_with_tx_hash_0():
    """Should fail on get_transaction_receipt with hash 0 without 0x prefix"""
    resp = get_transaction_receipt_test_client("0")
    assert resp.json["message"].startswith(INVALID_TRANSACTION_HASH_MESSAGE_PREFIX)
    assert resp.status_code == 500


@pytest.mark.parametrize("address_property", ["contract_address", "sender_address"])
def test_calling_with_different_address_properties(address_property: str):
    """In starknet 0.11 contract_address was changed to sender_address"""
    dummy_uninitialized_address = "0x01"
    resp = app.test_client().post(
        "/feeder_gateway/call_contract",
        data=json.dumps(
            {
                "entry_point_selector": "0x0",
                "calldata": [],
                "signature": [],
                address_property: dummy_uninitialized_address,
            }
        ),
    )

    assert resp.status_code == 500
    assert resp.is_json
    assert resp.json.get("code") == str(StarknetErrorCode.UNINITIALIZED_CONTRACT)


def test_calling_without_body():
    """Test graceful failing without body"""
    resp = app.test_client().post("/feeder_gateway/call_contract")
    assert resp.status_code == 500
    assert resp.is_json
    assert resp.json.get("code") == str(StarkErrorCode.MALFORMED_REQUEST)
