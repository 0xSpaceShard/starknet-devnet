"""Test old block support"""

from .account import invoke
from .shared import (
    ABI_PATH,
    CONTRACT_PATH,
    PREDEPLOY_ACCOUNT_CLI_ARGS,
    PREDEPLOYED_ACCOUNT_ADDRESS,
    PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
    SUFFICIENT_MAX_FEE,
)
from .util import assert_tx_status, call, deploy, devnet_in_background


@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS)
def test_call():
    """Expect variable value not to be changed in the old state"""

    def increment():
        # increase_balance accepts two args, but the second one is here fixed to 0 for simplicity
        tx_hash = invoke(
            calls=[(contract_address, "increase_balance", [increment_value, 0])],
            account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
            private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
            max_fee=SUFFICIENT_MAX_FEE,
        )
        assert_tx_status(tx_hash, "ACCEPTED_ON_L2")

    def get_value(block_number: str):
        value = call(
            "get_balance",
            address=contract_address,
            abi_path=ABI_PATH,
            block_number=block_number,
        )
        return int(value)

    def assert_old_block_correct():
        value_at_1 = get_value(block_number="1")
        assert value_at_1 == initial_value

        value_at_2 = get_value(block_number="2")
        assert value_at_2 == value_at_1 + increment_value

    initial_value = 5
    deploy_info = deploy(CONTRACT_PATH, inputs=[str(initial_value)])
    contract_address = deploy_info["address"]

    increment_value = 7
    increment()

    assert_old_block_correct()
    latest_value = get_value(block_number="latest")
    assert latest_value == initial_value + increment_value

    # generate another transaction to make the block/state older
    # and to change the value in the latest state
    increment()

    assert_old_block_correct()
