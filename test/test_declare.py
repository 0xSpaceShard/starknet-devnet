"""
Tests of contract class declaration and deploy syscall.
"""

import pytest

from .account import declare, invoke
from .shared import (
    ABI_PATH,
    CONTRACT_PATH,
    DEPLOYER_CONTRACT_PATH,
    EXPECTED_CLASS_HASH,
    PREDEPLOY_ACCOUNT_CLI_ARGS,
    PREDEPLOYED_ACCOUNT_ADDRESS,
    PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
)
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
)


def assert_deployed_through_syscall(tx_hash, initial_balance: str):
    """Asserts that a contract has been deployed using the deploy syscall"""
    assert_tx_status(tx_hash, "ACCEPTED_ON_L2")

    # Get deployment address from emitted event
    tx_receipt = get_transaction_receipt(tx_hash=tx_hash)
    events = tx_receipt["events"]
    event = events[0]
    assert_equal(len(event["data"]), 1, explanation=events)
    contract_address = event["data"][0]

    # Test deployed contract
    fetched_class_hash = get_class_hash_at(contract_address=contract_address)
    assert_hex_equal(fetched_class_hash, EXPECTED_CLASS_HASH)

    balance = call(function="get_balance", address=contract_address, abi_path=ABI_PATH)
    assert_equal(balance, initial_balance)


@pytest.mark.declare
@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS)
def test_declare_and_deploy():
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
    deployer_address = deployer_deploy_info["address"]

    assert_deployed_through_syscall(
        deployer_deploy_info["tx_hash"], initial_balance_in_constructor
    )

    # Deploy a contract of the declared class through the deployer
    initial_balance = "10"
    invoke_tx_hash = invoke(
        calls=[(deployer_address, "deploy_contract", [initial_balance])],
        account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
    )
    assert_deployed_through_syscall(invoke_tx_hash, initial_balance)
