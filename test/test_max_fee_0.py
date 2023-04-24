"""
Test max fee functionality.
"""

from starkware.starknet.definitions.error_codes import StarknetErrorCode

from .account import declare, declare_and_deploy_with_chargeable, invoke
from .shared import (
    CONTRACT_PATH,
    PREDEPLOY_ACCOUNT_CLI_ARGS,
    PREDEPLOYED_ACCOUNT_ADDRESS,
    PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
)
from .util import (
    ErrorExpector,
    assert_class_by_hash,
    assert_tx_status,
    devnet_in_background,
)


@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS)
def test_invoke_with_max_fee_0():
    """Test that invoke transaction with max fee 0 will fail"""
    initial_balance = 100
    deploy_info = declare_and_deploy_with_chargeable(
        CONTRACT_PATH, [str(initial_balance)]
    )
    account_address = PREDEPLOYED_ACCOUNT_ADDRESS
    private_key = PREDEPLOYED_ACCOUNT_PRIVATE_KEY
    calls = [(deploy_info["address"], "increase_balance", [10, 20])]
    with ErrorExpector(StarknetErrorCode.OUT_OF_RANGE_FEE):
        invoke(calls, account_address, private_key, max_fee=0)


@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS, "--allow-max-fee-zero")
def test_invoke_with_max_fee_0_and_allow_max_fee_zero():
    """Test that invoke transaction with max fee 0 will succeed if allow flag is set"""
    initial_balance = 100
    deploy_info = declare_and_deploy_with_chargeable(
        CONTRACT_PATH, [str(initial_balance)]
    )
    account_address = PREDEPLOYED_ACCOUNT_ADDRESS
    private_key = PREDEPLOYED_ACCOUNT_PRIVATE_KEY
    calls = [(deploy_info["address"], "increase_balance", [10, 20])]
    invoke_tx_hash = invoke(calls, account_address, private_key, max_fee=0)
    assert_tx_status(invoke_tx_hash, "ACCEPTED_ON_L2")


@devnet_in_background()
def test_declare_with_max_fee_0():
    """Test that declare transaction with max fee 0 will fail"""
    with ErrorExpector(StarknetErrorCode.OUT_OF_RANGE_FEE):
        declare(
            contract_path=CONTRACT_PATH,
            account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
            private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
            max_fee=0,
        )


@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS, "--allow-max-fee-zero")
def test_declare_with_max_fee_0_and_allow_max_fee_zero():
    """Test that declare transaction with max fee 0 will succeed if allow flag is set"""
    declare_info = declare(
        contract_path=CONTRACT_PATH,
        account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
        max_fee=0,
    )
    class_hash = declare_info["class_hash"]
    assert_class_by_hash(class_hash, CONTRACT_PATH)
