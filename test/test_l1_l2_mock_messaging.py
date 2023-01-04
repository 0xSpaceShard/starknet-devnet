"""
Test l1 l2 mock messaging.
"""

import requests
from starkware.starknet.definitions.error_codes import StarknetErrorCode
from starkware.starknet.public.abi import get_selector_from_name
from starkware.starkware_utils.error_handling import StarkErrorCode

from .account import invoke
from .settings import APP_URL
from .shared import (
    L1L2_ABI_PATH,
    L1L2_CONTRACT_PATH,
    PREDEPLOY_ACCOUNT_CLI_ARGS,
    PREDEPLOYED_ACCOUNT_ADDRESS,
    PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
)
from .util import (
    assert_tx_status,
    call,
    deploy,
    devnet_in_background,
    load_file_content,
)

DEPLOY_CONTENT = load_file_content("deploy.json")
INVOKE_CONTENT = load_file_content("invoke.json")
CALL_CONTENT = load_file_content("call.json")
INVALID_HASH = "0x58d4d4ed7580a7a98ab608883ec9fe722424ce52c19f2f369eeea301f535914"
INVALID_ADDRESS = "0x123"
USER_ID = 1
L1_CONTRACT_ADDRESS = 0xE7F1725E7734CE288F8367E1BB143E90BB3F0512
L2_CONTRACT_ADDRESS = (
    "0x00285ddb7e5c777b310d806b9b2a0f7c7ba0a41f12b420219209d97a3b7f25b2"
)
ENTRY_POINT_SELECTOR = hex(get_selector_from_name("deposit"))
MESSAGE_TO_L2_NONCE = "0x0"

# from_address, user and amount for L2 contract
CONSUME_PAYLOAD = ["0x0", "0x1", "0x3e8"]

# user and amount for L1 contract
MESSAGE_TO_L2_PAYLOAD = ["0x1", "0x1"]


def send_message_to_l2(req_dict: dict):
    """Sends the dict in a POST request and returns the response data."""
    return requests.post(
        f"{APP_URL}/postman/send_message_to_l2",
        json=req_dict,
    )


def consume_message_from_l2(req_dict: dict):
    """Sends the dict in a POST request and returns the response data."""
    return requests.post(
        f"{APP_URL}/postman/consume_message_from_l2",
        json=req_dict,
    )


@devnet_in_background()
def test_send_message_to_l2_deploy_execute():
    """Test POST l1 to l2 deploy contract and execute transaction"""
    # Deploy L1L2 contract
    deploy_info = deploy(contract=L1L2_CONTRACT_PATH)

    # Create l1 to l2 mock transaction
    response = send_message_to_l2(
        {
            "l2_contract_address": deploy_info["address"],
            "entry_point_selector": ENTRY_POINT_SELECTOR,
            "l1_contract_address": str(L1_CONTRACT_ADDRESS),
            "payload": MESSAGE_TO_L2_PAYLOAD,
            "nonce": MESSAGE_TO_L2_NONCE,
        }
    )

    # Check balance of user
    value = call(
        function="get_balance",
        address=deploy_info["address"],
        abi_path=L1L2_ABI_PATH,
        inputs=[str(USER_ID)],
    )
    assert int(value) == 1
    assert response.status_code == 200
    assert_tx_status(response.json().get("transaction_hash"), "ACCEPTED_ON_L2")


@devnet_in_background()
def test_send_message_to_l2_execute_without_data():
    """Test POST l1 to l2 without data"""
    # Create l1 to l2 mock transaction
    response = send_message_to_l2(
        {
            "l2_contract_address": "",
            "entry_point_selector": "",
            "l1_contract_address": "",
            "payload": "",
            "nonce": "",
        }
    )

    assert response.status_code == 400
    assert response.json().get("code") == str(StarkErrorCode.MALFORMED_REQUEST)


@devnet_in_background()
def test_send_message_to_l2_execute_without_deploy():
    """Test POST l1 to l2 without the target contract being deployed"""
    # Create l1 to l2 mock transaction
    response = send_message_to_l2(
        {
            "l2_contract_address": L2_CONTRACT_ADDRESS,
            "entry_point_selector": ENTRY_POINT_SELECTOR,
            "l1_contract_address": str(L1_CONTRACT_ADDRESS),
            "payload": MESSAGE_TO_L2_PAYLOAD,
            "nonce": MESSAGE_TO_L2_NONCE,
        }
    )

    assert response.status_code == 200
    assert_tx_status(response.json().get("transaction_hash"), "REJECTED")


@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS)
def test_consume_message_from_l2_deploy_execute():
    """Test POST l2 to l1 deploy contract and execute transaction"""
    deploy_info = deploy(L1L2_CONTRACT_PATH)

    # increase and withdraw balance
    invoke(
        calls=[(deploy_info["address"], "increase_balance", [USER_ID, 3333])],
        account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
    )
    invoke(
        calls=[
            (
                deploy_info["address"],
                "withdraw",
                [USER_ID, 1000, 0xE7F1725E7734CE288F8367E1BB143E90BB3F0512],
            )
        ],
        account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
    )

    response = consume_message_from_l2(
        {
            "l2_contract_address": deploy_info["address"],
            "l1_contract_address": str(L1_CONTRACT_ADDRESS),
            "payload": CONSUME_PAYLOAD,
        }
    )

    assert response.status_code == 200


@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS)
def test_consume_message_from_l2_deploy_execute_without_withdraw():
    """Test POST l2 to l1 deploy contract and try to execute transaction without calling withdraw"""
    deploy_info = deploy(L1L2_CONTRACT_PATH)
    response = consume_message_from_l2(
        {
            "l2_contract_address": deploy_info["address"],
            "l1_contract_address": str(L1_CONTRACT_ADDRESS),
            "payload": CONSUME_PAYLOAD,
        }
    )

    assert response.status_code == 500
    assert response.json().get("code") == str(
        StarknetErrorCode.L1_TO_L2_MESSAGE_ZEROED_COUNTER
    )


@devnet_in_background()
def test_consume_message_from_l2_execute_without_data():
    """Test POST l2 to l1 deploy without data"""
    response = consume_message_from_l2(
        {
            "l2_contract_address": "",
            "l1_contract_address": "",
            "payload": "",
        }
    )

    assert response.status_code == 400
    assert response.json().get("code") == str(StarkErrorCode.MALFORMED_REQUEST)


@devnet_in_background()
def test_consume_message_from_l2_execute_without_deploy():
    """Test POST l2 to l1 without contract deploy"""
    response = consume_message_from_l2(
        {
            "l2_contract_address": L2_CONTRACT_ADDRESS,
            "l1_contract_address": str(L1_CONTRACT_ADDRESS),
            "payload": CONSUME_PAYLOAD,
        }
    )

    assert response.status_code == 500
    assert response.json().get("code") == str(
        StarknetErrorCode.L1_TO_L2_MESSAGE_ZEROED_COUNTER
    )
