"""
Test account functionality.
"""
from test.settings import APP_URL

import requests
import pytest

from .shared import ABI_PATH, CONTRACT_PATH, EVENTS_CONTRACT_PATH
from .util import (
    assert_equal,
    assert_events,
    assert_tx_status,
    deploy,
    devnet_in_background,
    get_transaction_receipt,
    load_file_content,
    call,
    estimate_fee,
)
from .account import (
    ACCOUNT_ABI_PATH,
    PRIVATE_KEY,
    PUBLIC_KEY,
    deploy_account_contract,
    get_nonce,
    execute,
    get_estimated_fee,
)

INVOKE_CONTENT = load_file_content("invoke.json")
DEPLOY_CONTENT = load_file_content("deploy.json")
ACCOUNT_ADDRESS = "0x0555a7156bd44a6c4dba0cf819b8afe8dfdca5ec4cf56a8f5021d02752e63660"
INVALID_HASH = "0x58d4d4ed7580a7a98ab608883ec9fe722424ce52c19f2f369eeea301f535914"
SALT = "0x99"

ACCOUNTS_SEED_DEVNET_ARGS = [
    "--accounts",
    "1",
    "--seed",
    "42",
    "--gas-price",
    "100_000_000",
    "--initial-balance",
    "1_000_000_000_000_000_000_000",
]
PREDEPLOYED_ACCOUNT_ADDRESS = (
    "0x347be35996a21f6bf0623e75dbce52baba918ad5ae8d83b6f416045ab22961a"
)
PREDEPLOYED_ACCOUNT_PRIVATE_KEY = 0xBDD640FB06671AD11C80317FA3B1799D


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
    deploy_info = deploy_account_contract(salt=SALT)
    assert deploy_info["address"] == ACCOUNT_ADDRESS

    deployed_public_key = call("get_public_key", ACCOUNT_ADDRESS, ACCOUNT_ABI_PATH)
    assert int(deployed_public_key, 16) == PUBLIC_KEY

    nonce = get_nonce(ACCOUNT_ADDRESS)
    assert nonce == "0"


@pytest.mark.account
@devnet_in_background()
def test_invoking_another_contract():
    """Test invoking another contract."""
    deploy_info = deploy_empty_contract()
    deploy_account_contract(salt=SALT)
    to_address = int(deploy_info["address"], 16)

    # execute increase_balance call
    calls = [(to_address, "increase_balance", [10, 20])]
    tx_hash = execute(calls, ACCOUNT_ADDRESS, PRIVATE_KEY)

    assert_tx_status(tx_hash, "ACCEPTED_ON_L2")

    # check if nonce is increased
    nonce = get_nonce(ACCOUNT_ADDRESS)
    assert nonce == "1"

    # check if balance is increased
    balance_raw = execute(
        [(to_address, "get_balance", [])], ACCOUNT_ADDRESS, PRIVATE_KEY, query=True
    )
    balance_arr = balance_raw.split()
    assert_equal(len(balance_arr), 2)
    balance = balance_arr[1]
    assert balance == "30"


@pytest.mark.account
@devnet_in_background()
def test_estimated_fee():
    """Test estimate fees."""
    deploy_info = deploy_empty_contract()
    deploy_account_contract(salt=SALT)
    to_address = int(deploy_info["address"], 16)

    initial_balance = call("get_balance", deploy_info["address"], abi_path=ABI_PATH)

    # get estimated fee for increase_balance call
    calls = [(to_address, "increase_balance", [10, 20])]
    estimated_fee = get_estimated_fee(calls, ACCOUNT_ADDRESS, PRIVATE_KEY)

    assert estimated_fee > 0

    # estimate fee without account
    estimated_fee_without_account = estimate_fee(
        "increase_balance", ["10", "20"], deploy_info["address"], ABI_PATH
    )

    assert estimated_fee_without_account < estimated_fee

    # should not affect balance
    balance = call("get_balance", deploy_info["address"], abi_path=ABI_PATH)
    assert balance == initial_balance


@pytest.mark.account
@devnet_in_background()
def test_low_max_fee():
    """Test if transaction is rejected with low max fee"""
    deploy_info = deploy_empty_contract()
    deploy_account_contract(salt=SALT)
    to_address = int(deploy_info["address"], 16)

    initial_balance = call("get_balance", deploy_info["address"], abi_path=ABI_PATH)

    # get estimated fee for increase_balance call
    calls = [(to_address, "increase_balance", [10, 20])]
    estimated_fee = get_estimated_fee(calls, ACCOUNT_ADDRESS, PRIVATE_KEY)
    assert estimated_fee > 1

    tx_hash = execute(calls, ACCOUNT_ADDRESS, PRIVATE_KEY, max_fee=estimated_fee - 1)

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
    to_address = int(deploy_info["address"], 16)
    initial_account_balance = get_account_balance(account_address)

    initial_contract_balance = call(
        "get_balance", deploy_info["address"], abi_path=ABI_PATH
    )

    args = [10, 20]
    calls = [(to_address, "increase_balance", args)]
    estimated_fee = get_estimated_fee(calls, account_address, private_key)
    assert estimated_fee > 0

    invoke_tx_hash = execute(calls, account_address, private_key, max_fee=estimated_fee)
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
    "--accounts",
    "1",
    "--seed",
    "42",
    "--gas-price",
    "100_000_000",
    "--initial-balance",
    "10",
)
def test_insufficient_balance():
    """Test handling of insufficient account balance"""
    deploy_info = deploy_empty_contract()
    account_address = (
        "0x347be35996a21f6bf0623e75dbce52baba918ad5ae8d83b6f416045ab22961a"
    )
    private_key = 0xBDD640FB06671AD11C80317FA3B1799D
    to_address = int(deploy_info["address"], 16)
    initial_account_balance = get_account_balance(account_address)

    initial_contract_balance = call(
        "get_balance", deploy_info["address"], abi_path=ABI_PATH
    )

    args = [10, 20]
    calls = [(to_address, "increase_balance", args)]
    invoke_tx_hash = execute(
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
    deploy_account_contract(salt=SALT)
    to_address = int(deploy_info["address"], 16)

    # execute increase_balance calls
    calls = [
        (to_address, "increase_balance", [10, 20]),
        (to_address, "increase_balance", [30, 40]),
    ]
    tx_hash = execute(calls, ACCOUNT_ADDRESS, PRIVATE_KEY)

    assert_tx_status(tx_hash, "ACCEPTED_ON_L2")

    # check if nonce is increased
    nonce = get_nonce(ACCOUNT_ADDRESS)
    assert nonce == "1"

    # check if balance is increased
    balance = call("get_balance", deploy_info["address"], abi_path=ABI_PATH)
    assert balance == "100"


@pytest.mark.account
@devnet_in_background(*ACCOUNTS_SEED_DEVNET_ARGS)
def test_events():
    """Test transaction receipt events"""
    deploy_info = deploy_events_contract()
    deploy_account_contract(salt=SALT)
    account_address = PREDEPLOYED_ACCOUNT_ADDRESS
    private_key = PREDEPLOYED_ACCOUNT_PRIVATE_KEY
    to_address = int(deploy_info["address"], 16)

    args = [10]
    calls = [(to_address, "increase_balance", args)]
    estimated_fee = get_estimated_fee(calls, account_address, private_key)
    assert estimated_fee > 0

    invoke_tx_hash = execute(calls, account_address, private_key, max_fee=estimated_fee)
    assert_events(invoke_tx_hash, "test/expected/invoke_receipt_account_event.json")
