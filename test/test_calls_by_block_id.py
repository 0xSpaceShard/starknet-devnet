"""Test old block support"""

from starkware.starknet.definitions.error_codes import StarknetErrorCode

from .account import invoke
from .shared import (
    ABI_PATH,
    CONTRACT_PATH,
    PREDEPLOY_ACCOUNT_CLI_ARGS,
    PREDEPLOYED_ACCOUNT_ADDRESS,
    PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
    SUFFICIENT_MAX_FEE,
)
from .test_restart import restart
from .util import (
    ErrorExpector,
    call,
    demand_block_creation,
    deploy,
    devnet_in_background,
    get_block,
)


def _increment(contract_address: str, increment_value: int):
    # increase_balance accepts two args, but the second one is here fixed to 0 for simplicity
    invoke(
        calls=[(contract_address, "increase_balance", [increment_value, 0])],
        account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
        max_fee=SUFFICIENT_MAX_FEE,
    )


def _get_value(contract_address: str, block_number: str) -> int:
    value = call(
        "get_balance",
        address=contract_address,
        abi_path=ABI_PATH,
        block_number=block_number,
    )
    return int(value)


@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS)
def test_call():
    """Expect variable value not to be changed in the old state"""

    def assert_old_block_correct():
        value_at_1 = _get_value(contract_address, block_number="1")
        assert value_at_1 == initial_value

        value_at_2 = _get_value(contract_address, block_number="2")
        assert value_at_2 == value_at_1 + increment_value

    initial_value = 5
    deploy_info = deploy(CONTRACT_PATH, inputs=[str(initial_value)])
    contract_address = deploy_info["address"]

    increment_value = 7
    _increment(contract_address, increment_value)

    assert_old_block_correct()
    latest_value = _get_value(contract_address, block_number="latest")
    assert latest_value == initial_value + increment_value

    # generate another transaction to make the block/state older
    # and to change the value in the latest state
    _increment(contract_address, increment_value)

    assert_old_block_correct()


FORK_BLOCK = 1000


@devnet_in_background(
    *PREDEPLOY_ACCOUNT_CLI_ARGS,
    "--fork-network",
    "alpha-goerli",
    "--fork-block",
    str(FORK_BLOCK),
)
def test_forked():
    """Fork an origin. Fail if calling old, succeed if calling new state."""

    # devnet added a genesis block at FORK_BLOCK + 1

    initial_balance = 10
    deploy_info = deploy(contract=CONTRACT_PATH, inputs=[str(initial_balance)])
    contract_address = deploy_info["address"]

    first_increment_value = 7
    _increment(contract_address, increment_value=first_increment_value)
    _increment(contract_address, increment_value=5)

    latest_block = get_block(block_number="latest", parse=True)
    # genesis + deploy + invoke + invoke
    assert latest_block["block_number"] == FORK_BLOCK + 4

    with ErrorExpector(StarknetErrorCode.OUT_OF_RANGE_BLOCK_ID):
        # before first devnet block
        _get_value(contract_address, block_number=str(FORK_BLOCK))

    with ErrorExpector(StarknetErrorCode.UNINITIALIZED_CONTRACT):
        # at genesis block, but before deployment
        _get_value(contract_address, block_number=str(FORK_BLOCK + 1))

    value_after_deploy = _get_value(contract_address, block_number=str(FORK_BLOCK + 2))
    assert value_after_deploy == initial_balance

    value_after_first_invoke = _get_value(
        contract_address, block_number=str(FORK_BLOCK + 3)
    )
    assert value_after_first_invoke == initial_balance + first_increment_value


@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS)
def test_after_restart():
    """Call a state after calling restart - expect failure"""

    initial_balance = 5
    deploy_info = deploy(contract=CONTRACT_PATH, inputs=[str(initial_balance)])
    contract_address = deploy_info["address"]

    # first assert that it's callable before the restart
    assert _get_value(contract_address, block_number="1") == initial_balance

    restart()

    # assert not callable after the restart
    with ErrorExpector(StarknetErrorCode.UNINITIALIZED_CONTRACT):
        _get_value(contract_address, block_number="latest")


@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS, "--blocks-on-demand")
def test_old_block_generated_on_demand():
    """Call old blocks generated on demand"""

    initial_balance = 10
    deploy_info = deploy(contract=CONTRACT_PATH, inputs=[str(initial_balance)])
    contract_address = deploy_info["address"]

    increment_value = 5
    _increment(contract_address, increment_value)

    demand_block_creation()

    _increment(contract_address, increment_value)
    demand_block_creation()

    latest_block = get_block(block_number="latest", parse=True)
    assert latest_block["block_number"] == 2  # genesis (0) + demand + demand

    assert (
        _get_value(contract_address, block_number="latest")
        == initial_balance + 2 * increment_value
    )

    assert (
        _get_value(contract_address, block_number="1")
        == initial_balance + increment_value
    )