"""
Test account functionality.
"""

import requests
import pytest

from .settings import APP_URL
from .shared import (
    ABI_PATH,
    CONTRACT_PATH,
    EVENTS_CONTRACT_PATH,
    PREDEPLOY_ACCOUNT_CLI_ARGS,
    PREDEPLOYED_ACCOUNT_ADDRESS,
    PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
)
from .util import (
    assert_equal,
    assert_events,
    assert_transaction,
    assert_tx_status,
    deploy,
    devnet_in_background,
    get_transaction_receipt,
    load_file_content,
    call,
)
from .account import (
    ACCOUNT_ABI_PATH,
    PRIVATE_KEY,
    PUBLIC_KEY,
    deploy_account_contract,
    get_nonce,
    invoke,
    get_estimated_fee,
)

INVOKE_CONTENT = load_file_content("invoke.json")
DEPLOY_CONTENT = load_file_content("deploy.json")
SALTY_ACCOUNT_ADDRESS = (
    "0x030dec1363f9fc6ecf36be88845e43025861a851dce1439a61c1e7b148a9892b"
)
INVALID_HASH = "0x58d4d4ed7580a7a98ab608883ec9fe722424ce52c19f2f369eeea301f535914"
SALT = "0x99"

ACCOUNTS_SEED_DEVNET_ARGS = [
    *PREDEPLOY_ACCOUNT_CLI_ARGS,
    "--gas-price",
    "100_000_000",
    "--initial-balance",
    "1_000_000_000_000_000_000_000",
]


def deploy_empty_contract():
    """Deploy sample contract with balance = 0."""
    return deploy(CONTRACT_PATH, inputs=["0"], salt=SALT)


def deploy_events_contract():
    """Deploy events contract with salt of 0x99."""
    return deploy(EVENTS_CONTRACT_PATH, salt=SALT)


def get_account_balance(address: str) -> int:
    """Get balance (wei) of account with `address` (hex)."""
    resp = requests.get(f"{APP_URL}/account_balance?address={address}")
    assert resp.status_code == 200
    return int(resp.json()["amount"])


@pytest.mark.account
@devnet_in_background()
def test_account_contract_deploy():
    """Test account contract deploy, public key and initial nonce value."""
    account_deploy_info = deploy_account_contract(salt=SALT)
    account_address = account_deploy_info["address"]
    assert account_address == SALTY_ACCOUNT_ADDRESS

    deployed_public_key = call("getPublicKey", account_address, ACCOUNT_ABI_PATH)
    assert int(deployed_public_key, 16) == PUBLIC_KEY

    nonce = get_nonce(account_address)
    assert nonce == 0


@pytest.mark.account
@devnet_in_background()
def test_invoking_another_contract():
    """Test invoking another contract through a newly deployed (not predeployed) account."""
    deploy_info = deploy_empty_contract()
    account_address = deploy_account_contract(salt=SALT)["address"]
    to_address = deploy_info["address"]

    # execute increase_balance call
    calls = [(to_address, "increase_balance", [10, 20])]
    # setting max_fee=0 skips fee subtraction, otherwise account would need funds
    tx_hash = invoke(calls, account_address, PRIVATE_KEY, 0, max_fee=0)

    assert_tx_status(tx_hash, "ACCEPTED_ON_L2")

    # check if nonce is increased
    nonce = get_nonce(account_address)
    assert nonce == 1

    # check if balance is increased
    balance = call("get_balance", to_address, ABI_PATH, [])
    assert balance == "30"


@pytest.mark.account
@devnet_in_background()
def test_estimated_fee():
    """Test estimate fees."""
    deploy_info = deploy_empty_contract()
    account_address = deploy_account_contract(salt=SALT)["address"]

    initial_balance = call("get_balance", deploy_info["address"], abi_path=ABI_PATH)

    # get estimated fee for increase_balance call
    calls = [(deploy_info["address"], "increase_balance", [10, 20])]
    estimated_fee = get_estimated_fee(calls, account_address, PRIVATE_KEY)

    assert estimated_fee > 0

    # here we used to test estimation directly on contract, not supported anymore

    # should not affect balance
    balance = call("get_balance", deploy_info["address"], abi_path=ABI_PATH)
    assert balance == initial_balance


@pytest.mark.account
@devnet_in_background()
def test_low_max_fee():
    """Test if transaction is rejected with low max fee"""
    deploy_info = deploy_empty_contract()
    account_address = deploy_account_contract(salt=SALT)["address"]

    initial_balance = call("get_balance", deploy_info["address"], abi_path=ABI_PATH)

    # get estimated fee for increase_balance call
    calls = [(deploy_info["address"], "increase_balance", [10, 20])]
    estimated_fee = get_estimated_fee(calls, account_address, PRIVATE_KEY)
    assert estimated_fee > 1

    tx_hash = invoke(calls, account_address, PRIVATE_KEY, max_fee=estimated_fee - 1)

    assert_tx_status(tx_hash, "REJECTED")

    balance = call("get_balance", deploy_info["address"], abi_path=ABI_PATH)

    assert_equal(balance, initial_balance)


@pytest.mark.account
@devnet_in_background(*ACCOUNTS_SEED_DEVNET_ARGS)
def test_sufficient_max_fee():
    """Test invoking with a sufficient max fee."""
    deploy_info = deploy_empty_contract()
    account_address = PREDEPLOYED_ACCOUNT_ADDRESS
    private_key = PREDEPLOYED_ACCOUNT_PRIVATE_KEY
    initial_account_balance = get_account_balance(account_address)

    initial_contract_balance = call(
        "get_balance", deploy_info["address"], abi_path=ABI_PATH
    )

    args = [10, 20]
    calls = [(deploy_info["address"], "increase_balance", args)]
    estimated_fee = get_estimated_fee(calls, account_address, private_key)
    assert estimated_fee > 0

    invoke_tx_hash = invoke(calls, account_address, private_key, max_fee=estimated_fee)
    assert_tx_status(invoke_tx_hash, "ACCEPTED_ON_L2")

    invoke_receipt = get_transaction_receipt(invoke_tx_hash)
    actual_fee = int(invoke_receipt["actual_fee"], 16)

    final_contract_balance = call(
        "get_balance", deploy_info["address"], abi_path=ABI_PATH
    )
    assert_equal(int(final_contract_balance), int(initial_contract_balance) + sum(args))

    final_account_balance = get_account_balance(account_address)
    assert_equal(final_account_balance, initial_account_balance - actual_fee)


@pytest.mark.account
@devnet_in_background(
    *PREDEPLOY_ACCOUNT_CLI_ARGS,
    "--gas-price",
    "100_000_000",
    "--initial-balance",
    "10",
)
def test_insufficient_balance():
    """Test handling of insufficient account balance"""
    deploy_info = deploy_empty_contract()
    account_address = PREDEPLOYED_ACCOUNT_ADDRESS
    private_key = PREDEPLOYED_ACCOUNT_PRIVATE_KEY
    initial_account_balance = get_account_balance(account_address)

    initial_contract_balance = call(
        "get_balance", deploy_info["address"], abi_path=ABI_PATH
    )

    args = [10, 20]
    calls = [(deploy_info["address"], "increase_balance", args)]
    invoke_tx_hash = invoke(
        calls, account_address, private_key, max_fee=10**21
    )  # big enough

    assert_tx_status(invoke_tx_hash, "REJECTED")
    invoke_receipt = get_transaction_receipt(invoke_tx_hash)
    assert (
        "subtraction overflow"
        in invoke_receipt["transaction_failure_reason"]["error_message"]
    )

    final_contract_balance = call(
        "get_balance", deploy_info["address"], abi_path=ABI_PATH
    )
    assert_equal(final_contract_balance, initial_contract_balance)

    final_account_balance = get_account_balance(account_address)
    assert_equal(initial_account_balance, final_account_balance)


@pytest.mark.account
@devnet_in_background()
def test_multicall():
    """Test making multiple calls."""
    deploy_info = deploy_empty_contract()
    account_address = deploy_account_contract(salt=SALT)["address"]
    to_address = deploy_info["address"]

    # execute increase_balance calls
    calls = [
        (to_address, "increase_balance", [10, 20]),
        (to_address, "increase_balance", [30, 40]),
    ]
    # setting max_fee=0 skips fee subtraction, otherwise account would need funds
    tx_hash = invoke(calls, account_address, PRIVATE_KEY, max_fee=0)

    assert_tx_status(tx_hash, "ACCEPTED_ON_L2")

    # check if nonce is increased
    nonce = get_nonce(account_address)
    assert nonce == 1

    # check if balance is increased
    balance = call("get_balance", deploy_info["address"], abi_path=ABI_PATH)
    assert balance == "100"


@pytest.mark.account
@devnet_in_background(*ACCOUNTS_SEED_DEVNET_ARGS)
def test_events():
    """Test transaction receipt events"""
    deploy_info = deploy_events_contract()
    account_address = PREDEPLOYED_ACCOUNT_ADDRESS
    private_key = PREDEPLOYED_ACCOUNT_PRIVATE_KEY

    calls = [(deploy_info["address"], "increase_balance", [10])]
    estimated_fee = get_estimated_fee(calls, account_address, private_key)
    assert estimated_fee > 0

    invoke_tx_hash = invoke(calls, account_address, private_key, max_fee=estimated_fee)
    assert_events(invoke_tx_hash, "test/expected/invoke_receipt_account_event.json")


def get_nonce_with_request(address: str):
    """Do GET on /get_nonce for `address`"""
    return requests.get(f"{APP_URL}/feeder_gateway/get_nonce?contractAddress={address}")


@pytest.mark.account
@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS)
def test_get_nonce_endpoint():
    """Test get_nonce endpoint"""

    account_address = PREDEPLOYED_ACCOUNT_ADDRESS

    initial_resp = get_nonce_with_request(address=account_address)
    assert initial_resp.status_code == 200
    assert initial_resp.json() == "0x0"

    deployment_info = deploy_empty_contract()

    invoke_tx_hash = invoke(
        calls=[(deployment_info["address"], "increase_balance", [10, 20])],
        account_address=account_address,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
    )
    assert_transaction(invoke_tx_hash, "ACCEPTED_ON_L2")

    final_resp = get_nonce_with_request(address=account_address)
    assert final_resp.status_code == 200
    assert final_resp.json() == "0x1"
