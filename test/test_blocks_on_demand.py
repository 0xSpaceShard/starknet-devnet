"""
Test blocks on demand mode.
"""

from test.rpc.rpc_utils import gateway_call

import requests

from .account import invoke
from .settings import APP_URL
from .shared import (
    ABI_PATH,
    CONTRACT_PATH,
    PREDEPLOY_ACCOUNT_CLI_ARGS,
    PREDEPLOYED_ACCOUNT_ADDRESS,
    PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
)
from .util import ReturnCodeAssertionError, call, deploy, devnet_in_background


@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS, "--blocks-on-demand")
def test_blocks_on_demand_invoke():
    """Test deploy in blocks on demand mode"""
    latest_block = gateway_call("get_block", blockNumber="latest")
    genesis_block_number = latest_block["block_number"]
    assert genesis_block_number == 0

    deploy_info = deploy(CONTRACT_PATH, inputs=["0"])
    should_fail_on_contract_call = False
    try:
        call(
            function="get_balance",
            address=deploy_info["address"],
            abi_path=ABI_PATH,
        )
    except ReturnCodeAssertionError:
        should_fail_on_contract_call = True
    assert should_fail_on_contract_call is True

    invoke(
        calls=[(deploy_info["address"], "increase_balance", [10, 20])],
        account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
    )
    latest_block = gateway_call("get_block", blockNumber="latest")
    block_number_after_deploy_and_invoke = latest_block["block_number"]
    assert block_number_after_deploy_and_invoke == 0

    requests.post(f"{APP_URL}/create_block_on_demand")

    balance_after_create_block_on_demand = call(
        function="get_balance",
        address=deploy_info["address"],
        abi_path=ABI_PATH,
    )
    assert int(balance_after_create_block_on_demand) == 30

    latest_block = gateway_call("get_block", blockNumber="latest")
    block_number_after_block_on_demand_call = latest_block["block_number"]
    assert block_number_after_block_on_demand_call == 1
    assert len(latest_block["transactions"]) == 2


@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS, "--blocks-on-demand")
def test_blocks_on_demand_invoke_call():
    """
    Test deploy in blocks on demand mode for invoke and contract call.
    Balance after invoke should be 0 even when we increased it.
    """
    # Deploy and invoke
    deploy_info = deploy(CONTRACT_PATH, inputs=["0"])
    requests.post(f"{APP_URL}/create_block_on_demand")

    balance_after_deploy = call(
        function="get_balance",
        address=deploy_info["address"],
        abi_path=ABI_PATH,
    )
    assert int(balance_after_deploy) == 0

    invoke(
        calls=[(deploy_info["address"], "increase_balance", [10, 20])],
        account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
    )
    balance_after_invoke = call(
        function="get_balance",
        address=deploy_info["address"],
        abi_path=ABI_PATH,
    )
    assert int(balance_after_invoke) == 0

    requests.post(f"{APP_URL}/create_block_on_demand")
    balance_after_create_block_on_demand = call(
        function="get_balance",
        address=deploy_info["address"],
        abi_path=ABI_PATH,
    )
    assert int(balance_after_create_block_on_demand) == 30
