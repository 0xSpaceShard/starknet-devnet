"""Test devnet contract deployment"""

from typing import List

import pytest
from starkware.starknet.core.os.contract_address.contract_address import (
    calculate_contract_address,
    compute_class_hash,
)
from starkware.starknet.definitions.transaction_type import TransactionType
from starkware.starknet.public.abi import get_selector_from_name
from starkware.starknet.services.api.contract_class import ContractClass
from starkware.starknet.services.api.gateway.transaction import Deploy
from starkware.starknet.services.api.feeder_gateway.response_objects import (
    TransactionStatus,
)
from starkware.starknet.wallets.open_zeppelin import (
    sign_deploy_account_tx,
    sign_invoke_tx,
)
from starkware.starknet.definitions.general_config import DEFAULT_CHAIN_ID
from starkware.starknet.third_party.open_zeppelin.starknet_contracts import (
    account_contract as oz_account_class,
)
from starknet_devnet.constants import OZ_ACCOUNT_CLASS_HASH
from starknet_devnet.devnet_config import parse_args, DevnetConfig
from starknet_devnet.starknet_wrapper import StarknetWrapper
from starknet_devnet.udc import UDC

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

from .account import declare, invoke
from .util import (
    assert_contract_class,
    assert_equal,
    assert_hex_equal,
    assert_tx_status,
    call,
    deploy,
    devnet_in_background,
    get_class_by_hash,
    get_class_hash_at,
    get_transaction_receipt,
    load_contract_class,
    mint,
    send_tx,
)


def get_contract_class():
    """Get the contract class from the contract.json file."""
    with open(CONTRACT_PATH, "r", encoding="utf-8") as contract_class_file:
        return ContractClass.loads(contract_class_file.read())


def get_deploy_transaction(inputs: List[int], salt=0):
    """Get a Deploy transaction."""
    contract_class = get_contract_class()

    return Deploy(
        contract_address_salt=salt,
        contract_definition=contract_class,
        constructor_calldata=inputs,
        version=SUPPORTED_TX_VERSION,
    )


@pytest.fixture(name="starknet_wrapper_args")
def fixture_starknet_wrapper_args(request):
    """
    Fixture to return values of dev net arguments
    """
    return request.param


@pytest.mark.parametrize(
    "starknet_wrapper_args, expected_tx_hash, expected_block_hash",
    [
        (
            [*PREDEPLOY_ACCOUNT_CLI_ARGS],
            "0x13d4b9f765587296a4f40591efe235a8caf24f0496230f0b13a87f2e4c8150a",
            "",
        ),
        (
            [*PREDEPLOY_ACCOUNT_CLI_ARGS, "--lite-mode"],
            "0x0",
            "0x1",
        ),
    ],
    indirect=True,
)
@pytest.mark.asyncio
async def test_deploy(starknet_wrapper_args, expected_tx_hash, expected_block_hash):
    """
    Test the deployment of a contract.
    """
    devnet = StarknetWrapper(config=DevnetConfig(parse_args(starknet_wrapper_args)))
    await devnet.initialize()
    deploy_transaction = get_deploy_transaction(inputs=[0])

    contract_address, tx_hash = await devnet.deploy(
        deploy_transaction=deploy_transaction,
    )
    expected_contract_address = calculate_contract_address(
        deployer_address=0,
        constructor_calldata=deploy_transaction.constructor_calldata,
        salt=deploy_transaction.contract_address_salt,
        contract_class=deploy_transaction.contract_definition,
    )

    assert_hex_equal(
        hex(tx_hash),
        expected_tx_hash,
    )
    assert contract_address == expected_contract_address

    tx_status = devnet.transactions.get_transaction_status(hex(tx_hash))
    assert tx_status["tx_status"] == TransactionStatus.ACCEPTED_ON_L2.name

    if "--lite-mode" in starknet_wrapper_args:
        assert tx_status["block_hash"] == expected_block_hash


def test_predeployed_oz_account():
    """Test that precomputed class matches"""
    assert OZ_ACCOUNT_CLASS_HASH == compute_class_hash(oz_account_class)


@devnet_in_background()
def test_deploy_account():
    """Test the deployment of an account."""

    # the account class should already be declared

    # generate account with random keys and salt
    private_key = 0x6F9E0F15B20753CE2E2B740B182099C4ADF765D0C5A5B75C1AF3327358FBF2E
    public_key = 0x7707342F75277F32F1A0AD532E1A12016B36A3967332D31F915C889678B3DB6
    account_salt = 0x75B567ECB69C6D032982FA32C8F52D2F00DB50C5DE2C93EDDA70DE9B5109F8F
    account_address = calculate_contract_address(
        salt=account_salt,
        contract_class=oz_account_class,
        constructor_calldata=[public_key],
        deployer_address=0,
    )

    # prepare deploy account tx
    deploy_account_tx = sign_deploy_account_tx(
        private_key=private_key,
        public_key=public_key,
        class_hash=compute_class_hash(oz_account_class),
        salt=account_salt,
        max_fee=int(1e18),
        version=SUPPORTED_TX_VERSION,
        chain_id=DEFAULT_CHAIN_ID.value,
        nonce=0,
    ).dump()

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
    contract_deploy_info = deploy(contract=CONTRACT_PATH, inputs=[str(init_balance)])
    contract_address = contract_deploy_info["address"]

    # increase balance of test contract
    invoke_tx = sign_invoke_tx(
        signer_address=account_address,
        private_key=private_key,
        contract_address=int(contract_address, 16),
        selector=get_selector_from_name("increase_balance"),
        calldata=[10, 20],
        chain_id=DEFAULT_CHAIN_ID.value,
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


def _assert_deployed_through_syscall(
    tx_hash: str, address_index: int, initial_balance: str
):
    """Asserts that a contract has been deployed using the deploy syscall"""
    assert_tx_status(tx_hash, "ACCEPTED_ON_L2")

    # Get deployment address from emitted event
    tx_receipt = get_transaction_receipt(tx_hash=tx_hash)
    events = tx_receipt["events"]

    # there can be multiple events, e.g. from fee_token, but the first one is ours
    event = events[0]
    contract_address = event["data"][address_index]

    # Test deployed contract
    fetched_class_hash = get_class_hash_at(contract_address=contract_address)
    assert_hex_equal(fetched_class_hash, EXPECTED_CLASS_HASH)

    balance = call(function="get_balance", address=contract_address, abi_path=ABI_PATH)
    assert_equal(balance, initial_balance)


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
    )
    class_hash = declare_info["class_hash"]
    assert_hex_equal(class_hash, EXPECTED_CLASS_HASH)

    contract_class = get_class_by_hash(class_hash=class_hash)
    assert_contract_class(contract_class, CONTRACT_PATH)

    # Deploy the deployer - also deploys a contract of the declared class using the deploy syscall
    initial_balance_in_constructor = "5"
    deployer_deploy_info = deploy(
        contract=DEPLOYER_CONTRACT_PATH,
        inputs=[class_hash, initial_balance_in_constructor],
    )

    _assert_deployed_through_syscall(
        tx_hash=deployer_deploy_info["tx_hash"],
        address_index=0,
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


@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS)
def test_deploy_with_udc():
    """Test if deploying through UDC works."""

    # Declare the class to be deployed
    declare_info = declare(
        contract_path=CONTRACT_PATH,
        account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
    )
    class_hash = declare_info["class_hash"]
    assert_hex_equal(class_hash, EXPECTED_CLASS_HASH)

    contract_class = get_class_by_hash(class_hash=class_hash)
    assert_contract_class(contract_class, CONTRACT_PATH)

    # Deploy a contract of the declared class through the deployer
    initial_balance = "10"
    ctor_args = [initial_balance]
    invoke_tx_hash = invoke(
        calls=[
            (
                EXPECTED_UDC_ADDRESS,
                "deployContract",
                [
                    int(class_hash, 16),
                    42,  # salt
                    0,  # unique
                    len(ctor_args),
                    *ctor_args,
                ],
            )
        ],
        account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
    )
    _assert_deployed_through_syscall(
        tx_hash=invoke_tx_hash,
        address_index=0,
        initial_balance=initial_balance,
    )
