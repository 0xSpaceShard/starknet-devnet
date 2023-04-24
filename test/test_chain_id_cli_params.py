"""Testing chain id CLI params"""

import subprocess

import pytest
from starkware.starknet.definitions.general_config import StarknetChainId

from starknet_devnet.devnet_config import CHAIN_IDS

from .account import declare_and_deploy_with_chargeable, invoke
from .shared import (
    ABI_PATH,
    CONTRACT_PATH,
    PREDEPLOY_ACCOUNT_CLI_ARGS,
    PREDEPLOYED_ACCOUNT_ADDRESS,
    PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
)
from .util import (
    DevnetBackgroundProc,
    assert_equal,
    assert_transaction,
    assert_tx_status,
    call,
    read_stream,
    terminate_and_wait,
)

ACTIVE_DEVNET = DevnetBackgroundProc()


@pytest.mark.parametrize(
    "chain_id",
    [member.name for member in StarknetChainId],
)
def test_chain_id_valid(chain_id):
    """Test if chain id works"""
    proc = ACTIVE_DEVNET.start(
        "--chain-id",
        chain_id,
    )
    terminate_and_wait(proc)
    assert proc.returncode == 0


@pytest.mark.parametrize(
    "chain_id",
    ["", "mainnet2"],
)
def test_chain_id_invalid(chain_id):
    """Test if the invalid chain id fails"""
    proc = ACTIVE_DEVNET.start(
        "--chain-id",
        chain_id,
        stderr=subprocess.PIPE,
        stdout=subprocess.PIPE,
    )
    assert (
        f"Error: The value of --chain-id must be in {{{CHAIN_IDS}}}, got:"
        in read_stream(proc.stderr)
    )
    assert proc.returncode == 1


@pytest.mark.usefixtures("run_devnet_in_background")
@pytest.mark.parametrize(
    "run_devnet_in_background, chain_id",
    [
        ([*PREDEPLOY_ACCOUNT_CLI_ARGS, "--chain-id", chain_id.name], chain_id)
        for chain_id in StarknetChainId
    ],
    indirect=True,
)
def test_deploy_and_invoke(chain_id):
    """Test deploy and invoke with MAINNET and TESTNET chain_id"""
    deploy_info = declare_and_deploy_with_chargeable(
        CONTRACT_PATH, inputs=["0"], chain_id=chain_id
    )

    assert_tx_status(deploy_info["tx_hash"], "ACCEPTED_ON_L2")
    assert_transaction(deploy_info["tx_hash"], "ACCEPTED_ON_L2")

    # increase and assert balance
    invoke_tx_hash = invoke(
        calls=[(deploy_info["address"], "increase_balance", [10, 20])],
        account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
        chain_id=chain_id,
    )
    assert_transaction(invoke_tx_hash, "ACCEPTED_ON_L2")

    value = call(
        function="get_balance", address=deploy_info["address"], abi_path=ABI_PATH
    )
    assert_equal(value, "30", "Invoke+call failed!")
