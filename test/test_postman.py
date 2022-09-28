"""
Test postman usage. This test has one single pytest case, because the whole flow needs to be tested, and requires all steps to be performed
"""

import json
import subprocess

import psutil
import pytest

from web3 import Web3
import requests

from .account import invoke
from .util import (
    call,
    deploy,
    devnet_in_background,
    ensure_server_alive,
    estimate_message_fee,
    load_file_content,
    terminate_and_wait,
)
from .settings import APP_URL, L1_HOST, L1_PORT, L1_URL
from .shared import (
    L1L2_ABI_PATH,
    L1L2_CONTRACT_PATH,
    PREDEPLOY_ACCOUNT_CLI_ARGS,
    PREDEPLOYED_ACCOUNT_ADDRESS,
    PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
)
from .web3_util import web3_call, web3_deploy, web3_transact

ETH_CONTRACTS_PATH = "artifacts/contracts/solidity"
STARKNET_MESSAGING_PATH = (
    f"{ETH_CONTRACTS_PATH}/MockStarknetMessaging.sol/MockStarknetMessaging.json"
)
L1L2_EXAMPLE_PATH = f"{ETH_CONTRACTS_PATH}/L1L2.sol/L1L2Example.json"

USER_ID = 1


@pytest.fixture(autouse=True)
def run_before_and_after_test():
    """Run l1 testnet before and kill it after the test run"""
    # Setup L1 testnet

    command = ["npx", "hardhat", "node", "--hostname", L1_HOST, "--port", L1_PORT]
    with subprocess.Popen(command, close_fds=True) as node_proc:
        # before test
        ensure_server_alive(L1_URL, node_proc)

        yield

        # after test
        wrapped_node_proc = psutil.Process(node_proc.pid)
        children = wrapped_node_proc.children(recursive=True)
        print("Killing children", children)
        for child_proc in children:
            terminate_and_wait(child_proc)
        print("Children after killing", wrapped_node_proc.children(recursive=True))
        terminate_and_wait(node_proc)


def flush():
    """Flushes the postman messages. Returns response data"""
    res = requests.post(f"{APP_URL}/postman/flush")

    return res.json()


def assert_flush_response(
    response, expected_from_l1, expected_from_l2, expected_l1_provider
):
    """Asserts that the flush response is correct"""

    assert response["l1_provider"] == expected_l1_provider

    for i, l1_message in enumerate(response["consumed_messages"]["from_l1"]):
        assert (
            l1_message["args"]["from_address"]
            == expected_from_l1[i]["args"]["from_address"]
        )
        assert (
            l1_message["args"]["to_address"]
            == expected_from_l1[i]["args"]["to_address"]
        )
        assert l1_message["args"]["payload"] == [
            hex(x) for x in expected_from_l1[i]["args"]["payload"]
        ]

        # check if correct keys are present
        expected_keys = [
            "block_hash",
            "block_number",
            "transaction_hash",
            "transaction_index",
            "address",
            "event",
            "log_index",
        ]

        for key in expected_keys:
            assert key in l1_message

        expected_args_keys = ["selector", "nonce"]

        for key in expected_args_keys:
            assert key in l1_message["args"]

    for i, l2_message in enumerate(response["consumed_messages"]["from_l2"]):
        assert l2_message["from_address"] == expected_from_l2[i]["from_address"].lower()
        assert l2_message["to_address"] == expected_from_l2[i]["to_address"].lower()
        assert l2_message["payload"] == [hex(x) for x in expected_from_l2[i]["payload"]]


def init_messaging_contract():
    """Initializes the messaging contract"""

    deploy_messaging_contract_request = {"networkUrl": L1_URL}
    resp = requests.post(
        f"{APP_URL}/postman/load_l1_messaging_contract",
        json=deploy_messaging_contract_request,
    )
    return json.loads(resp.text)


def deploy_l1_contracts(web3):
    """Deploys Ethereum contracts in the Hardhat testnet instance, including the L1L2Example and MockStarknetMessaging contracts"""

    messaging_contract = json.loads(load_file_content(STARKNET_MESSAGING_PATH))
    l1l2_example_contract = json.loads(load_file_content(L1L2_EXAMPLE_PATH))

    # Min amount of time in seconds for a message to be able to be cancelled
    l1_message_cancellation_delay = 0
    # Deploys a new mock contract so that the feature for loading an already deployed messaging contract can be tested
    starknet_messaging_contract = web3_deploy(
        web3, messaging_contract, l1_message_cancellation_delay
    )
    l1l2_example = web3_deploy(
        web3, l1l2_example_contract, starknet_messaging_contract.address
    )

    return starknet_messaging_contract, l1l2_example


def load_messaging_contract(starknet_messaging_contract_address):
    """Loads a Mock Messaging contract already deployed in the local testnet instance"""

    load_messaging_contract_request = {
        "networkUrl": L1_URL,
        "address": starknet_messaging_contract_address,
    }

    resp = requests.post(
        f"{APP_URL}/postman/load_l1_messaging_contract",
        json=load_messaging_contract_request,
    )

    return json.loads(resp.text)


def _init_l2_contract(l1l2_example_contract_address: str):
    """Deploys the L1L2Example cairo contract, returns the result of calling 'get_balance'"""

    deploy_info = deploy(L1L2_CONTRACT_PATH)
    l2_address = deploy_info["address"]

    # increase and withdraw balance
    invoke(
        calls=[(l2_address, "increase_balance", [USER_ID, 3333])],
        account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
    )
    contract_address_int = int(l1l2_example_contract_address, 16)
    invoke(
        calls=[(l2_address, "withdraw", [USER_ID, 1000, contract_address_int])],
        account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
    )

    # flush L2 to L1 messages
    flush_response = flush()

    assert_flush_response(
        response=flush_response,
        expected_from_l1=[],
        expected_from_l2=[
            {
                "from_address": deploy_info["address"],
                "to_address": l1l2_example_contract_address,
                "payload": [0, USER_ID, 1000],  # MESSAGE_WITHDRAW, user, amount
            }
        ],
        expected_l1_provider=L1_URL,
    )

    # assert balance
    value = call(
        function="get_balance",
        address=deploy_info["address"],
        abi_path=L1L2_ABI_PATH,
        inputs=[str(USER_ID)],
    )

    assert value == "2333"
    return deploy_info["address"]


def _l1_l2_message_exchange(web3, l1l2_example_contract, l2_contract_address):
    """Tests message exchange"""

    # assert contract balance when starting
    balance = web3_call("userBalances", l1l2_example_contract, USER_ID)
    assert balance == 0

    # withdraw in l1 and assert contract balance
    web3_transact(
        web3,
        "withdraw",
        l1l2_example_contract,
        int(l2_contract_address, base=16),
        USER_ID,
        1000,
    )

    balance = web3_call("userBalances", l1l2_example_contract, USER_ID)
    assert balance == 1000

    # assert l2 contract balance
    l2_balance = call(
        function="get_balance",
        address=l2_contract_address,
        abi_path=L1L2_ABI_PATH,
        inputs=[str(USER_ID)],
    )
    assert l2_balance == "2333"

    message_fee = estimate_message_fee(
        from_address=l1l2_example_contract.address,
        function="deposit",
        inputs=[str(USER_ID), "100"],
        to_address=l2_contract_address,
        abi_path=L1L2_ABI_PATH,
    )
    assert int(message_fee) > 0

    # deposit in l1 and assert contract balance
    web3_transact(
        web3,
        "deposit",
        l1l2_example_contract,
        int(l2_contract_address, base=16),
        USER_ID,
        600,
    )

    balance = web3_call("userBalances", l1l2_example_contract, USER_ID)

    assert balance == 400

    # flush L1 to L2 messages
    flush_response = flush()

    assert_flush_response(
        response=flush_response,
        expected_from_l1=[
            {
                "address": None,
                "args": {
                    "from_address": l1l2_example_contract.address,
                    "to_address": l2_contract_address,
                    "payload": [USER_ID, 600],  # user, amount
                },
            }
        ],
        expected_from_l2=[],
        expected_l1_provider=L1_URL,
    )

    # assert l2 contract balance
    l2_balance = call(
        function="get_balance",
        address=l2_contract_address,
        abi_path=L1L2_ABI_PATH,
        inputs=[str(USER_ID)],
    )

    assert l2_balance == "2933"


@pytest.mark.web3_messaging
@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS)
def test_postman():
    """Test postman with a complete L1<>L2 flow"""
    l1l2_example_contract = None
    starknet_messaging_contract = None
    l2_contract_address = None
    web3 = None

    # Test initializing a local L1 network
    init_resp = init_messaging_contract()
    web3 = Web3(Web3.HTTPProvider(L1_URL))
    web3.eth.default_account = web3.eth.accounts[0]
    assert "address" in init_resp
    assert init_resp["l1_provider"] == L1_URL

    starknet_messaging_contract, l1l2_example_contract = deploy_l1_contracts(web3)

    # Test loading the messaging contract
    load_resp = load_messaging_contract(starknet_messaging_contract.address)
    assert load_resp["address"] == starknet_messaging_contract.address
    assert load_resp["l1_provider"] == L1_URL

    # Test initializing the l2 example contract
    l2_contract_address = _init_l2_contract(l1l2_example_contract.address)

    _l1_l2_message_exchange(web3, l1l2_example_contract, l2_contract_address)


def _load_l1_messaging_contract(req_dict: dict):
    """Load L1 messaging contract"""
    return requests.post(
        f"{APP_URL}/postman/load_l1_messaging_contract", json=(req_dict)
    )


@pytest.mark.web3_messaging
@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS)
def test_invalid_starknet_function_call_load_l1_messaging_contract():
    """Call with invalid data on starknet function call"""
    load_messaging_contract_request = {}
    resp = _load_l1_messaging_contract(load_messaging_contract_request)

    json_error_message = resp.json()["message"]
    msg = "L1 network or StarknetMessaging contract address not specified"
    assert resp.status_code == 400
    assert msg in json_error_message
