"""Test old block support"""

from starkware.starknet.definitions.error_codes import StarknetErrorCode

from .account import declare_and_deploy_with_chargeable, invoke
from .shared import (
    ABI_PATH,
    BALANCE_KEY,
    CONTRACT_PATH,
    PREDEPLOY_ACCOUNT_CLI_ARGS,
    PREDEPLOYED_ACCOUNT_ADDRESS,
    PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
    SUFFICIENT_MAX_FEE,
)
from .test_restart import restart
from .util import (
    ErrorExpector,
    assert_storage,
    call,
    demand_block_creation,
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
        # genesis (0) + declare + deploy = block number 2
        value_at_1 = _get_value(contract_address, block_number="2")
        assert value_at_1 == initial_value

        # genesis (0) + declare + deploy + invoke = block number 3
        value_at_2 = _get_value(contract_address, block_number="3")
        assert value_at_2 == value_at_1 + increment_value

    initial_value = 5
    deploy_info = declare_and_deploy_with_chargeable(
        CONTRACT_PATH, inputs=[str(initial_value)]
    )
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


@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS)
def test_call_with_block_hash():
    """Expect variable value not to be changed in the old state"""

    def _get_value_by_hash(contract_address: str, block_hash: str) -> int:
        value = call(
            "get_balance",
            address=contract_address,
            abi_path=ABI_PATH,
            block_hash=block_hash,
        )
        return int(value)

    def assert_old_block_correct():
        value_at_1 = _get_value_by_hash(
            contract_address, block_hash=deployment_block_hash
        )
        assert value_at_1 == initial_value

        value_at_2 = _get_value_by_hash(
            contract_address, block_hash=increment_block_hash
        )
        assert value_at_2 == value_at_1 + increment_value

    initial_value = 5
    deploy_info = declare_and_deploy_with_chargeable(
        CONTRACT_PATH, inputs=[str(initial_value)]
    )
    deployment_block = get_block(block_number="latest", parse=True)
    deployment_block_hash = deployment_block["block_hash"]
    contract_address = deploy_info["address"]

    increment_value = 7
    _increment(contract_address, increment_value)
    increment_block = get_block(block_number="latest", parse=True)
    increment_block_hash = increment_block["block_hash"]
    assert increment_block_hash != deployment_block_hash

    assert_old_block_correct()

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
    deploy_info = declare_and_deploy_with_chargeable(
        contract=CONTRACT_PATH, inputs=[str(initial_balance)]
    )
    contract_address = deploy_info["address"]

    first_increment_value = 7
    _increment(contract_address, increment_value=first_increment_value)
    _increment(contract_address, increment_value=5)

    latest_block = get_block(block_number="latest", parse=True)
    # fork + genesis + declare + deploy + invoke + invoke
    assert latest_block["block_number"] == FORK_BLOCK + 5

    with ErrorExpector(StarknetErrorCode.OUT_OF_RANGE_BLOCK_ID):
        # before first devnet block
        _get_value(contract_address, block_number=str(FORK_BLOCK))

    with ErrorExpector(StarknetErrorCode.UNINITIALIZED_CONTRACT):
        # at genesis block, but before deployment
        _get_value(contract_address, block_number=str(FORK_BLOCK + 2))

    value_after_deploy = _get_value(contract_address, block_number=str(FORK_BLOCK + 3))
    assert value_after_deploy == initial_balance

    value_after_first_invoke = _get_value(
        contract_address, block_number=str(FORK_BLOCK + 4)
    )
    assert value_after_first_invoke == initial_balance + first_increment_value


@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS)
def test_after_restart():
    """Call a state after calling restart - expect failure"""

    initial_balance = 5
    deploy_info = declare_and_deploy_with_chargeable(
        contract=CONTRACT_PATH, inputs=[str(initial_balance)]
    )
    contract_address = deploy_info["address"]

    # first assert that it's callable before the restart
    assert _get_value(contract_address, block_number="latest") == initial_balance

    restart()

    # assert not callable after the restart
    with ErrorExpector(StarknetErrorCode.UNINITIALIZED_CONTRACT):
        _get_value(contract_address, block_number="latest")


@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS, "--blocks-on-demand")
def test_old_block_generated_on_demand():
    """Call old blocks generated on demand"""

    initial_balance = 10
    deploy_info = declare_and_deploy_with_chargeable(
        contract=CONTRACT_PATH, inputs=[str(initial_balance)]
    )
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


@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS)
def test_getting_storage_at_old_block():
    """Call get_storage_at on old block"""

    initial_balance = 10
    deploy_info = declare_and_deploy_with_chargeable(
        contract=CONTRACT_PATH, inputs=[str(initial_balance)]
    )
    deployment_block = get_block(block_number="latest", parse=True)
    contract_address = deploy_info["address"]

    increment_value = 5
    _increment(contract_address, increment_value)

    def assert_balance_in_storage(
        expected_value: str, block_number=None, block_hash=None
    ):
        assert_storage(
            address=contract_address,
            key=BALANCE_KEY,
            expected_value=expected_value,
            block_number=block_number,
            block_hash=block_hash,
        )

    # declaration block
    assert_balance_in_storage(expected_value=hex(0), block_number="1")
    assert_balance_in_storage(expected_value=hex(initial_balance), block_number="2")
    assert_balance_in_storage(
        expected_value=hex(initial_balance),
        block_hash=deployment_block["block_hash"],
    )
    assert_balance_in_storage(
        expected_value=hex(initial_balance + increment_value),
        block_number="3",
    )
