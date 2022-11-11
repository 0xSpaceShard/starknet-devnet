"""
Test the forking feature.
Relying on the fact that devnet doesn't support specifying which block to query
"""

from .account import invoke
from .shared import (
    ABI_PATH,
    CONTRACT_PATH,
    PREDEPLOY_ACCOUNT_CLI_ARGS,
    PREDEPLOYED_ACCOUNT_ADDRESS,
    PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
)
from .settings import bind_free_port, HOST
from .util import assert_tx_status, call, deploy, devnet_in_background

ORIGIN_PORT, ORIGIN_URL = bind_free_port(HOST)
FORK_PORT, FORK_URL = bind_free_port(HOST)


@devnet_in_background("--port", ORIGIN_PORT, *PREDEPLOY_ACCOUNT_CLI_ARGS)
@devnet_in_background("--port", FORK_PORT, "--fork-network", ORIGIN_URL)
def test_origin_not_changed_if_fork_changed():
    """Invoke on fork, assert origin unchanged"""

    initial_balance = "10"
    deploy_info = deploy(
        contract=CONTRACT_PATH,
        inputs=[initial_balance],
        gateway_url=ORIGIN_URL,
    )
    contract_address = deploy_info["address"]

    invoke_tx_hash = invoke(
        calls=[(contract_address, "increase_balance", [1, 2])],
        account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
        gateway_url=FORK_URL,
    )

    origin_balance_after = call(
        function="get_balance",
        abi_path=ABI_PATH,
        address=contract_address,
        feeder_gateway_url=ORIGIN_URL,
    )
    assert origin_balance_after == initial_balance
    assert_tx_status(invoke_tx_hash, "NOT_RECEIVED", feeder_gateway_url=ORIGIN_URL)

    fork_balance_after = call(
        function="get_balance",
        abi_path=ABI_PATH,
        address=contract_address,
        feeder_gateway_url=FORK_URL,
    )
    assert fork_balance_after == "13"
    assert_tx_status(invoke_tx_hash, "ACCEPTED_ON_L2", feeder_gateway_url=FORK_URL)


@devnet_in_background("--fork-network", "alpha-goerli-2")
def test_fork_not_changed_if_origin_changed():
    """Invoke on origin, assert fork unchanged"""

    # contract expected to be already deployed


# TODO fork from alpha-goerli

# TODO rely on account in origin (and in fork)
