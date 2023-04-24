"""Test devnet contract deployment"""

import pytest
from starkware.starknet.core.os.contract_address.contract_address import (
    calculate_contract_address,
)
from starkware.starknet.core.os.contract_class.deprecated_class_hash import (
    compute_deprecated_class_hash,
)
from starkware.starknet.definitions.general_config import DEFAULT_CHAIN_ID
from starkware.starknet.definitions.transaction_type import TransactionType
from starkware.starknet.public.abi import get_selector_from_name
from starkware.starknet.services.api.contract_class.contract_class import (
    DeprecatedCompiledClass,
)
from starkware.starknet.third_party.open_zeppelin.starknet_contracts import (
    account_contract as oz_account_class,
)
from starkware.starknet.wallets.open_zeppelin import (
    sign_deploy_account_tx,
    sign_invoke_tx,
)

from starknet_devnet.constants import STARKNET_CLI_ACCOUNT_CLASS_HASH
from starknet_devnet.udc import UDC

from .account import declare, declare_and_deploy_with_chargeable
from .shared import (
    ABI_PATH,
    CONTRACT_PATH,
    DEPLOYER_CONTRACT_PATH,
    EXPECTED_CLASS_HASH,
    EXPECTED_UDC_ADDRESS,
    PREDEPLOY_ACCOUNT_CLI_ARGS,
    PREDEPLOYED_ACCOUNT_ADDRESS,
    PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
    STARKNET_CLI_ACCOUNT_ABI_PATH,
    SUPPORTED_TX_VERSION,
)
from .util import (
    assert_class_by_hash,
    assert_equal,
    assert_hex_equal,
    assert_tx_status,
    call,
    devnet_in_background,
    get_class_hash_at,
    get_transaction_receipt,
    load_contract_class,
    mint,
    send_tx,
)


def get_contract_class():
    """Get the contract class from the contract.json file."""
    with open(CONTRACT_PATH, "r", encoding="utf-8") as contract_class_file:
        return DeprecatedCompiledClass.loads(contract_class_file.read())


@pytest.fixture(name="starknet_wrapper_args")
def fixture_starknet_wrapper_args(request):
    """
    Fixture to return values of dev net arguments
    """
    return request.param


def test_predeclared_oz_account():
    """Test that precomputed class matches"""
    assert STARKNET_CLI_ACCOUNT_CLASS_HASH == compute_deprecated_class_hash(
        oz_account_class
    )


@devnet_in_background()
def test_deploy_account():
    """Test the deployment of an account."""
    deploy_account_test_body()


def deploy_account_test_body():
    """The body of account deployment test."""

    # the account class should already be declared

    # generate account with random keys and salt
    private_key = 0x6F9E0F15B20753CE2E2B740B182099C4ADF765D0C5A5B75C1AF3327358FBF2E
    public_key = 0x7707342F75277F32F1A0AD532E1A12016B36A3967332D31F915C889678B3DB6
    account_salt = 0x75B567ECB69C6D032982FA32C8F52D2F00DB50C5DE2C93EDDA70DE9B5109F8F

    # prepare deploy account tx
    account_address, deploy_account_tx = sign_deploy_account_tx(
        private_key=private_key,
        public_key=public_key,
        class_hash=compute_deprecated_class_hash(oz_account_class),
        salt=account_salt,
        max_fee=int(1e18),
        version=SUPPORTED_TX_VERSION,
        chain_id=DEFAULT_CHAIN_ID,
        nonce=0,
    )
    deploy_account_tx = deploy_account_tx.dump()

    # deployment should fail if no funds
    tx_before = send_tx(deploy_account_tx, TransactionType.DEPLOY_ACCOUNT)
    assert_tx_status(tx_before["transaction_hash"], "REJECTED")

    # fund the address of the account
    mint(hex(account_address), amount=int(1e18))

    # deploy the account
    tx_after = send_tx(deploy_account_tx, TransactionType.DEPLOY_ACCOUNT)
    assert_tx_status(tx_after["transaction_hash"], "ACCEPTED_ON_L2")

    # assert that contract can be interacted with
    retrieved_public_key = call(
        function="get_public_key",
        address=hex(account_address),
        abi_path=STARKNET_CLI_ACCOUNT_ABI_PATH,
    )
    assert int(retrieved_public_key, 16) == public_key

    # deploy a contract for testing
    init_balance = 10
    contract_deploy_info = declare_and_deploy_with_chargeable(
        contract=CONTRACT_PATH, inputs=[init_balance]
    )
    contract_address = contract_deploy_info["address"]

    # increase balance of test contract
    invoke_tx = sign_invoke_tx(
        signer_address=account_address,
        private_key=private_key,
        contract_address=int(contract_address, 16),
        selector=get_selector_from_name("increase_balance"),
        calldata=[10, 20],
        chain_id=DEFAULT_CHAIN_ID,
        max_fee=int(1e18),
        version=SUPPORTED_TX_VERSION,
        nonce=1,
    ).dump()

    invoke_tx = send_tx(invoke_tx, TransactionType.INVOKE_FUNCTION)
    assert_tx_status(invoke_tx["transaction_hash"], "ACCEPTED_ON_L2")

    # get balance of test contract
    balance_after = call(
        function="get_balance", address=contract_address, abi_path=ABI_PATH
    )
    assert balance_after == "40"


def assert_deployed_through_syscall(tx_hash: str, initial_balance: int):
    """Asserts that a contract has been deployed using the deploy syscall"""
    assert_tx_status(tx_hash, "ACCEPTED_ON_L2")

    # Get deployment address from emitted event
    tx_receipt = get_transaction_receipt(tx_hash=tx_hash)
    events = tx_receipt["events"]

    # there can be multiple events, e.g. from fee_token, but the first one is ours
    event = events[0]
    address_index = 0  # position in UDC event
    contract_address = event["data"][address_index]

    # Test deployed contract
    fetched_class_hash = get_class_hash_at(contract_address=contract_address)
    assert_hex_equal(fetched_class_hash, EXPECTED_CLASS_HASH)

    balance = call(function="get_balance", address=contract_address, abi_path=ABI_PATH)
    assert_equal(int(balance), initial_balance)


@pytest.mark.declare
@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS)
def test_deploy_through_deployer_constructor():
    """
    Test declaring a class and deploying it through an account.
    """

    # Declare the class to be deployed
    declare_info = declare(
        contract_path=CONTRACT_PATH,
        account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
        max_fee=int(4e16),
    )
    class_hash = declare_info["class_hash"]
    assert_hex_equal(class_hash, EXPECTED_CLASS_HASH)

    assert_class_by_hash(class_hash, CONTRACT_PATH)

    # Deploy the deployer - also deploys a contract of the declared class using the deploy syscall
    initial_balance_in_constructor = 5
    deployer_deploy_info = declare_and_deploy_with_chargeable(
        contract=DEPLOYER_CONTRACT_PATH,
        inputs=[int(class_hash, 16), initial_balance_in_constructor],
    )

    assert_deployed_through_syscall(
        tx_hash=deployer_deploy_info["tx_hash"],
        initial_balance=initial_balance_in_constructor,
    )


def test_precomputed_udc_address():
    """Test if the precomputed address of UDC is correct."""
    udc_contract_class = load_contract_class("starknet_devnet/UDC_OZ_0.5.0.json")
    calculated_address = calculate_contract_address(
        salt=0,
        contract_class=udc_contract_class,
        constructor_calldata=[],
        deployer_address=0,
    )
    assert_equal(UDC.ADDRESS, calculated_address)
    assert_equal(UDC.ADDRESS, int(EXPECTED_UDC_ADDRESS, 16))


@devnet_in_background()
def test_declare_and_deploy_happy_path():
    """Test if deploying through UDC works. Declare beforehand."""
    initial_balance = 10
    deploy_info = declare_and_deploy_with_chargeable(
        contract=CONTRACT_PATH, inputs=[initial_balance], salt=hex(42)
    )

    assert_deployed_through_syscall(
        tx_hash=deploy_info["tx_hash"],
        initial_balance=initial_balance,
    )


@devnet_in_background()
def test_declare_and_deploy_with_invalid_constructor_args():
    """Pass invalid constructor args. Expect failure."""
    invalid_constructor_args = []  # empty when it shouldn't be
    deploy_info = declare_and_deploy_with_chargeable(
        contract=CONTRACT_PATH,
        inputs=invalid_constructor_args,
        max_fee=int(1e18),  # prevent estimateFee - fails due to invalid args
    )

    assert_tx_status(deploy_info["tx_hash"], "REJECTED")
