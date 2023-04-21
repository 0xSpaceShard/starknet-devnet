"""
Test account functionality.
"""

import pytest
import requests
from starkware.crypto.signature.signature import private_to_stark_key

from .account import (
    ACCOUNT_ABI_PATH,
    declare_and_deploy,
    declare_and_deploy_with_chargeable,
    deploy_account_contract,
    get_estimated_fee,
    get_nonce,
    invoke,
)
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
    assert_hex_equal,
    assert_transaction,
    assert_tx_status,
    call,
    devnet_in_background,
    get_transaction_receipt,
    mint,
)

SALTY_ACCOUNT_ADDRESS = (
    "0x025b4639c321f3538c69537502f0379a23d6c91d6bf0f9dfe79fabfc3da97de2"
)
SALT = "0x99"

ACCOUNTS_SEED_DEVNET_ARGS = [
    *PREDEPLOY_ACCOUNT_CLI_ARGS,
    "--gas-price",
    "100_000_000",
    "--initial-balance",
    "1_000_000_000_000_000_000_000",
]

PRIVATE_KEY = 123456789987654321
PUBLIC_KEY = private_to_stark_key(PRIVATE_KEY)


def deploy_empty_contract():
    """
    Deploy sample contract with balance = 0.
    This function expects to be called when running devnet in background with the usual seed
    """
    deploy_info = declare_and_deploy_with_chargeable(
        contract=CONTRACT_PATH,
        inputs=[0],
        salt=SALT,
    )
    assert_tx_status(deploy_info["tx_hash"], "ACCEPTED_ON_L2")
    return deploy_info


def deploy_events_contract():
    """Deploy events contract with salt of 0x99."""
    return declare_and_deploy_with_chargeable(
        contract=EVENTS_CONTRACT_PATH,
        salt=SALT,
    )


def get_account_balance(address: str, server_url=APP_URL) -> int:
    """Get balance (wei) of account with `address` (hex)."""
    resp = requests.get(f"{server_url}/account_balance?address={address}")
    assert resp.status_code == 200, resp.json()
    return int(resp.json()["amount"])


@pytest.mark.account
@devnet_in_background()
def test_account_contract_deploy():
    """Test account contract deploy, public key and initial nonce value."""
    mint(address=SALTY_ACCOUNT_ADDRESS, amount=int(1e18), lite=True)
    account_deploy_info = deploy_account_contract(private_key=PRIVATE_KEY, salt=SALT)
    account_address = account_deploy_info["address"]
    assert_hex_equal(account_address, SALTY_ACCOUNT_ADDRESS)

    deployed_public_key = call("getPublicKey", account_address, ACCOUNT_ABI_PATH)
    assert int(deployed_public_key, 16) == PUBLIC_KEY

    nonce = get_nonce(account_address)
    assert nonce == 1  # tested on alpha-goerli2: nonce is 1 right after deployment


@pytest.mark.account
@devnet_in_background()
def test_invoking_another_contract():
    """Test invoking another contract through a newly deployed (not predeployed) account."""
    # deploy the non-account contract
    deploy_info = deploy_empty_contract()
    to_address = deploy_info["address"]

    mint(address=SALTY_ACCOUNT_ADDRESS, amount=int(1e18), lite=True)
    deploy_account_info = deploy_account_contract(private_key=PRIVATE_KEY, salt=SALT)
    assert_tx_status(deploy_account_info["tx_hash"], "ACCEPTED_ON_L2")
    account_address = deploy_account_info["address"]
    assert_hex_equal(account_address, SALTY_ACCOUNT_ADDRESS)

    # execute increase_balance call
    calls = [(to_address, "increase_balance", [10, 20])]
    tx_hash = invoke(calls, account_address, PRIVATE_KEY)

    assert_tx_status(tx_hash, "ACCEPTED_ON_L2")

    # check if nonce is increased
    nonce = get_nonce(account_address)
    assert nonce == 2

    # check if balance is increased
    balance = call("get_balance", to_address, ABI_PATH, [])
    assert balance == "30"


@pytest.mark.account
@devnet_in_background()
def test_invoking_with_invalid_args():
    """Provide insufficient args. Expect failure."""

    deploy_info = deploy_empty_contract()
    to_address = deploy_info["address"]

    mint(address=SALTY_ACCOUNT_ADDRESS, amount=int(1e18), lite=True)
    deploy_account_info = deploy_account_contract(private_key=PRIVATE_KEY, salt=SALT)
    assert_tx_status(deploy_account_info["tx_hash"], "ACCEPTED_ON_L2")
    account_address = deploy_account_info["address"]
    assert_hex_equal(account_address, SALTY_ACCOUNT_ADDRESS)

    nonce_before = get_nonce(account_address)

    # execute increase_balance call
    invalid_args = [10]  # one param mising
    calls = [(to_address, "increase_balance", invalid_args)]
    tx_hash = invoke(
        calls,
        account_address=account_address,
        private_key=PRIVATE_KEY,
        max_fee=int(1e18),  # prevent estimateFee - fails due to invalid args
    )

    assert_tx_status(tx_hash, "REJECTED")

    # check if nonce is increased
    nonce_after = get_nonce(account_address)
    assert nonce_after == nonce_before


@pytest.mark.account
@devnet_in_background()
def test_estimated_fee():
    """Test estimate fees."""
    deploy_info = deploy_empty_contract()
    mint(address=SALTY_ACCOUNT_ADDRESS, amount=int(1e18), lite=True)
    deploy_account_info = deploy_account_contract(private_key=PRIVATE_KEY, salt=SALT)
    account_address = deploy_account_info["address"]
    assert_hex_equal(account_address, SALTY_ACCOUNT_ADDRESS)

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

    mint(address=SALTY_ACCOUNT_ADDRESS, amount=int(1e18), lite=True)
    deploy_account_info = deploy_account_contract(private_key=PRIVATE_KEY, salt=SALT)
    account_address = deploy_account_info["address"]
    assert_hex_equal(account_address, SALTY_ACCOUNT_ADDRESS)

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


def _assert_subtraction_overflow(tx_hash: str):
    assert_tx_status(tx_hash, "REJECTED")
    invoke_receipt = get_transaction_receipt(tx_hash)
    assert (
        "subtraction overflow"
        in invoke_receipt["transaction_failure_reason"]["error_message"]
    )


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
    )  # big enough to fail

    _assert_subtraction_overflow(invoke_tx_hash)

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
    to_address = deploy_info["address"]

    mint(address=SALTY_ACCOUNT_ADDRESS, amount=int(1e18), lite=True)
    deploy_account_info = deploy_account_contract(private_key=PRIVATE_KEY, salt=SALT)
    account_address = deploy_account_info["address"]
    assert_hex_equal(account_address, SALTY_ACCOUNT_ADDRESS)

    # execute increase_balance calls
    calls = [
        (to_address, "increase_balance", [10, 20]),
        (to_address, "increase_balance", [30, 40]),
    ]
    tx_hash = invoke(calls, account_address, PRIVATE_KEY, max_fee=int(1e18))

    assert_tx_status(tx_hash, "ACCEPTED_ON_L2")

    # check if nonce is increased
    nonce = get_nonce(account_address)
    assert nonce == 2

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

    deployment_info = declare_and_deploy(
        contract=CONTRACT_PATH,
        account_address=account_address,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
        inputs=[0],
    )

    final_resp = get_nonce_with_request(address=account_address)
    assert final_resp.status_code == 200
    assert final_resp.json() == "0x2"  # declare and deploy

    invoke_tx_hash = invoke(
        calls=[(deployment_info["address"], "increase_balance", [10, 20])],
        account_address=account_address,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
    )
    assert_transaction(invoke_tx_hash, "ACCEPTED_ON_L2")

    final_resp = get_nonce_with_request(address=account_address)
    assert final_resp.status_code == 200
    assert final_resp.json() == "0x3"  # invoke
