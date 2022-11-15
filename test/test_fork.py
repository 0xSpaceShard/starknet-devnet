"""
Test the forking feature.
Relying on the fact that devnet doesn't support specifying which block to query
"""

import pytest
import requests

from starkware.starknet.definitions.error_codes import StarknetErrorCode

from .account import invoke
from .shared import (
    ABI_PATH,
    ALPHA_GOERLI_2_URL,
    CONTRACT_PATH,
    PREDEPLOY_ACCOUNT_CLI_ARGS,
    PREDEPLOYED_ACCOUNT_ADDRESS,
    PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
)
from .settings import APP_URL, bind_free_port, HOST
from .util import (
    assert_tx_status,
    call,
    deploy,
    devnet_in_background,
)

ORIGIN_PORT, ORIGIN_URL = bind_free_port(HOST)
FORK_PORT, FORK_URL = bind_free_port(HOST)


def _invoke_on_fork_and_assert_only_fork_changed(
    contract_address: str,
    initial_balance: str,
    fork_url: str,
    origin_url: str,
):

    increase_args = [1, 2]
    invoke_tx_hash = invoke(
        calls=[(contract_address, "increase_balance", increase_args)],
        account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
        gateway_url=fork_url,
    )

    origin_balance_after = call(
        function="get_balance",
        abi_path=ABI_PATH,
        address=contract_address,
        feeder_gateway_url=origin_url,
    )
    assert origin_balance_after == initial_balance
    assert_tx_status(invoke_tx_hash, "NOT_RECEIVED", feeder_gateway_url=origin_url)

    fork_balance_after = call(
        function="get_balance",
        abi_path=ABI_PATH,
        address=contract_address,
        feeder_gateway_url=fork_url,
    )
    expected_balancer_after = str(int(initial_balance) + sum(increase_args))
    assert fork_balance_after == expected_balancer_after
    assert_tx_status(invoke_tx_hash, "ACCEPTED_ON_L2", feeder_gateway_url=fork_url)


def _deploy_on_origin_invoke_on_fork_assert_only_fork_changed(
    fork_url: str,
    origin_url: str,
    initial_balance="10",
):

    deploy_info = deploy(
        contract=CONTRACT_PATH,
        inputs=[initial_balance],
        gateway_url=origin_url,
    )

    _invoke_on_fork_and_assert_only_fork_changed(
        contract_address=deploy_info["address"],
        initial_balance=initial_balance,
        fork_url=fork_url,
        origin_url=origin_url,
    )


@devnet_in_background("--port", ORIGIN_PORT, *PREDEPLOY_ACCOUNT_CLI_ARGS)
@devnet_in_background(
    "--port", FORK_PORT, "--fork-network", ORIGIN_URL, "--accounts", "0"
)
def test_forking_devnet_with_account_on_origin():
    """
    Deploy contract on origin, invoke on fork, rely on account on origin.
    Assert only fork changed
    """

    _deploy_on_origin_invoke_on_fork_assert_only_fork_changed(
        fork_url=FORK_URL,
        origin_url=ORIGIN_URL,
    )


@devnet_in_background("--port", ORIGIN_PORT, "--accounts", "0")
@devnet_in_background(
    "--port", FORK_PORT, "--fork-network", ORIGIN_URL, *PREDEPLOY_ACCOUNT_CLI_ARGS
)
def test_forking_devnet_with_account_on_fork():
    """
    Deploy contract on origin, invoke on fork, rely on account on fork.
    Assert only fork changed
    """

    _deploy_on_origin_invoke_on_fork_assert_only_fork_changed(
        fork_url=FORK_URL,
        origin_url=ORIGIN_URL,
    )


TESTNET_URL = ALPHA_GOERLI_2_URL
TESTNET_CONTRACT_ADDRESS = (
    "0x32320dbdff79639db4ac0ff1f9f8b7450d31fee8ca1bccea7cfa0d7765fe0b2"
)
TESTNET_DEPLOYMENT_BLOCK = 8827  # this is when the contract was deployed
TESTNET_FORK_PARAMS = [*PREDEPLOY_ACCOUNT_CLI_ARGS, "--fork-network", "alpha-goerli-2"]


@pytest.mark.usefixtures("run_devnet_in_background")
@pytest.mark.parametrize(
    "run_devnet_in_background",
    [
        [*TESTNET_FORK_PARAMS, "--fork-block", str(TESTNET_DEPLOYMENT_BLOCK)],
        [*TESTNET_FORK_PARAMS, "--fork-block", str(TESTNET_DEPLOYMENT_BLOCK + 1)],
        [*TESTNET_FORK_PARAMS, "--fork-block", "latest"],
        [*TESTNET_FORK_PARAMS],  # should default to latest
    ],
    indirect=True,
)
def test_forking_testnet_from_valid_block():
    """Test forking from various happy path blocks"""

    _invoke_on_fork_and_assert_only_fork_changed(
        contract_address=TESTNET_CONTRACT_ADDRESS,
        initial_balance="10",
        fork_url=APP_URL,
        origin_url=TESTNET_URL,
    )


@devnet_in_background(
    *TESTNET_FORK_PARAMS, "--fork-block", str(TESTNET_DEPLOYMENT_BLOCK - 1)
)
def test_forking_testnet_from_too_early_block():
    """Test forking testnet if not yet deployed"""

    invoke_tx_hash = invoke(
        calls=[(TESTNET_CONTRACT_ADDRESS, "increase_balance", [1, 2])],
        account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
        max_fee=int(1e8),  # to prevent implicit fee estimation
    )

    assert_tx_status(invoke_tx_hash, "REJECTED", feeder_gateway_url=TESTNET_URL)
    class_resp = requests.get(
        f"{APP_URL}/feeder_gateway/get_class_hash_at",
        {"contractAddress": TESTNET_CONTRACT_ADDRESS},
    )

    assert class_resp.json()["code"] == str(StarknetErrorCode.UNINITIALIZED_CONTRACT)
    assert class_resp.status_code == 500


# TODO deploy on fork

# TODO test other feeder gateway responses

# TODO add test which asserts balance after tx
