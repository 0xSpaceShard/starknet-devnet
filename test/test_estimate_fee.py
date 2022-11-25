"""Fee estimation tests"""

import json
import typing

import pytest
import requests
from starkware.starknet.definitions.error_codes import StarknetErrorCode
from starkware.starknet.public.abi import get_selector_from_name
from starkware.starknet.services.api.feeder_gateway.response_objects import (
    FeeEstimationInfo,
)
from starkware.starknet.services.api.gateway.transaction import AccountTransaction

from starknet_devnet.constants import DEFAULT_GAS_PRICE

from .account import get_nonce
from .sample_tx_objects import TX_DICT1, TX_DICT2
from .settings import APP_URL
from .shared import (
    ABI_PATH,
    CONTRACT_PATH,
    EXPECTED_CLASS_HASH,
    EXPECTED_FEE_TOKEN_ADDRESS,
    L1L2_ABI_PATH,
    L1L2_CONTRACT_PATH,
    PREDEPLOY_ACCOUNT_CLI_ARGS,
    PREDEPLOYED_ACCOUNT_ADDRESS,
)
from .util import (
    call,
    deploy,
    devnet_in_background,
    estimate_message_fee,
    load_file_content,
)

DEPLOY_CONTENT = load_file_content("deploy.json")
INVOKE_CONTENT = load_file_content("invoke.json")


def estimate_fee_local(req_dict: dict):
    """Estimate fee of a given transaction"""
    return requests.post(f"{APP_URL}/feeder_gateway/estimate_fee", json=req_dict)


def send_estimate_fee_with_requests(req_dict: dict):
    """Sends the estimate fee dict in a POST request and returns the response data."""
    return requests.post(f"{APP_URL}/feeder_gateway/estimate_fee", json=req_dict)


def send_simulate_tx_with_requests(req_dict: dict):
    """Sends the simulate tx dict in a POST request and returns the response data."""
    return requests.post(
        f"{APP_URL}/feeder_gateway/simulate_transaction", json=req_dict
    )


def common_estimate_response(
    fee_estimation_info: typing.Union[dict, FeeEstimationInfo]
):
    """expected response from estimate_fee request"""

    if not isinstance(fee_estimation_info, FeeEstimationInfo):
        fee_estimation_info = FeeEstimationInfo.load(fee_estimation_info)

    assert fee_estimation_info.gas_price == DEFAULT_GAS_PRICE
    assert fee_estimation_info.gas_usage > 0
    expected_overall_fee = fee_estimation_info.gas_price * fee_estimation_info.gas_usage
    assert expected_overall_fee == fee_estimation_info.overall_fee
    assert fee_estimation_info.unit == "wei"


@devnet_in_background()
def test_estimate_fee_with_genesis_block():
    """Call without transaction, expect pass with gas_price zero"""
    response = send_estimate_fee_with_requests(
        {
            "entry_point_selector": "0x2f0b3c5710379609eb5495f1ecd348cb28167711b73609fe565a72734550354",
            "calldata": [
                "1786654640273905855542517570545751199272449814774211541121677632577420730552",
                "1000000000000000000000",
                "0",
            ],
            "signature": [],
            "contract_address": EXPECTED_FEE_TOKEN_ADDRESS,
        }
    )

    assert response.status_code == 200
    common_estimate_response(response.json())


@pytest.mark.estimate_fee
@devnet_in_background()
def test_estimate_fee_in_unknown_address():
    """Call with unknown invoke function"""
    req_dict = json.loads(INVOKE_CONTENT)
    del req_dict["type"]
    resp = estimate_fee_local(req_dict)

    json_error_message = resp.json()["message"]
    assert resp.status_code == 500
    assert json_error_message.endswith("is not deployed.")


@pytest.mark.estimate_fee
@devnet_in_background()
def test_estimate_fee_with_invalid_data():
    """Call estimate fee with invalid data on body"""
    req_dict = json.loads(DEPLOY_CONTENT)
    resp = estimate_fee_local(req_dict)

    json_error_message = resp.json()["message"]
    assert resp.status_code == 400
    assert "Invalid format of fee estimation request" in json_error_message


@pytest.mark.estimate_fee
@pytest.mark.parametrize(
    "request_kwargs",
    [
        {},  # tx version 0
        {"type": "INVOKE_FUNCTION"},  # tx version 1
    ],
)
@devnet_in_background("--gas-price", str(DEFAULT_GAS_PRICE))
def test_estimate_fee_with_complete_request_data(request_kwargs):
    """Estimate fee with complete request data"""

    deploy_info = deploy(CONTRACT_PATH, inputs=["0"])
    # increase balance with 10+20
    response = send_estimate_fee_with_requests(
        {
            "contract_address": deploy_info["address"],
            "version": "0x100000000000000000000000000000000",
            "signature": [],
            "calldata": ["10", "20"],
            "max_fee": "0x0",
            "entry_point_selector": "0x362398bec32bc0ebb411203221a35a0301193a96f317ebe5e40be9f60d15320",
            **request_kwargs,
        }
    )

    assert response.status_code == 200, f"Request not OK: {response.json()}"
    common_estimate_response(response.json())


@devnet_in_background("--gas-price", str(DEFAULT_GAS_PRICE))
def test_simulate_transaction():
    """Simulate tx"""

    deploy_info = deploy(CONTRACT_PATH, inputs=["0"])

    calldata = ["10", "20"]
    entry_point_selector = hex(get_selector_from_name("increase_balance"))
    response = send_simulate_tx_with_requests(
        {
            "contract_address": deploy_info["address"],
            "version": "0x100000000000000000000000000000000",
            "signature": [],
            "calldata": calldata,
            "max_fee": "0x0",
            "entry_point_selector": entry_point_selector,
            "type": "INVOKE_FUNCTION",
        }
    )

    response_body = response.json()
    assert response.status_code == 200, response_body
    common_estimate_response(response_body["fee_estimation"])

    assert response_body["trace"]["function_invocation"]
    assert response_body["trace"] == {
        "function_invocation": {
            "call_type": "CALL",
            "calldata": [hex(int(piece)) for piece in calldata],
            "caller_address": "0x0",
            "class_hash": EXPECTED_CLASS_HASH,
            "contract_address": hex(int(deploy_info["address"], 16)),
            "entry_point_type": "EXTERNAL",
            "events": [],
            "execution_resources": {
                "builtin_instance_counter": {},
                "n_memory_holes": 0,
                "n_steps": 67,
            },
            "internal_calls": [],
            "messages": [],
            "result": [],
            "selector": entry_point_selector,
        },
        "signature": [],
    }


@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS)
def test_estimate_message_fee():
    """Estimate message fee from l1 to l2"""

    dummy_l1_address = "0x1"
    user_id = "1"

    l2_contract_address = deploy(contract=L1L2_CONTRACT_PATH)["address"]

    message_fee = estimate_message_fee(
        from_address=dummy_l1_address,
        function="deposit",
        inputs=[user_id, "100"],
        to_address=l2_contract_address,
        abi_path=L1L2_ABI_PATH,
    )
    assert int(message_fee) > 0

    balance_after = call(
        function="get_balance",
        address=l2_contract_address,
        abi_path=L1L2_ABI_PATH,
        inputs=[user_id],
    )
    assert int(balance_after) == 0


def _send_estimate_fee_bulk_request(txs: typing.List[AccountTransaction]):
    return requests.post(
        f"{APP_URL}/feeder_gateway/estimate_fee_bulk",
        json=AccountTransaction.Schema().dump(txs, many=True),
    )


@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS)
def test_estimate_fee_bulk_invalid():
    """Test estimating fee in a bulk when one tx is invalid"""
    # skip deployment to cause failure

    tx_dicts = [TX_DICT1, TX_DICT2]
    txs = AccountTransaction.Schema().load(tx_dicts, many=True)

    resp = _send_estimate_fee_bulk_request(txs)
    assert resp.json()["code"] == str(StarknetErrorCode.UNINITIALIZED_CONTRACT)
    assert resp.status_code == 500


@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS)
def test_estimate_fee_bulk():
    """Test estimating fee in a bulk"""

    # contract must be deployed for fee estimation to be possible
    initial_balance = "10"
    deploy_info = deploy(contract=CONTRACT_PATH, inputs=[initial_balance], salt="0x42")

    tx_dicts = [TX_DICT1, TX_DICT2]  # two invokes
    # assert that loading can be done (i.e. object is structured correctly)
    txs = AccountTransaction.Schema().load(tx_dicts, many=True)

    resp = _send_estimate_fee_bulk_request(txs)
    assert resp.status_code == 200
    fee_estimation_infos = FeeEstimationInfo.Schema().load(resp.json(), many=True)

    # assert correct structure of response
    assert len(fee_estimation_infos) == len(tx_dicts) == 2
    for fee_estimation_info in fee_estimation_infos:
        common_estimate_response(fee_estimation_info)

    # tx at index 0 increases balance with [0, 0]
    assert fee_estimation_infos[0].gas_usage < fee_estimation_infos[1].gas_usage

    # assert no change done to contract
    nonce_after = get_nonce(account_address=PREDEPLOYED_ACCOUNT_ADDRESS)
    assert nonce_after == 0

    balance_after = call(
        function="get_balance", address=deploy_info["address"], abi_path=ABI_PATH
    )
    assert balance_after == initial_balance
