"""
Test blocks on demand mode.
"""

import requests
from test.rpc.rpc_utils import gateway_call

from .account import invoke
from .settings import APP_URL
from .shared import (
    CONTRACT_PATH,
    PREDEPLOY_ACCOUNT_CLI_ARGS,
    PREDEPLOYED_ACCOUNT_ADDRESS,
    PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
)
from .util import (
    deploy,
    devnet_in_background,
)

CONTRAC_HASH_CLASS = "0x71df7c871d389943e24aaaf85d41594266d12f2f9b580a9f92ba4a0bf763d67"


@devnet_in_background(*[*PREDEPLOY_ACCOUNT_CLI_ARGS,"--blocks-on-demand",])
def test_blocks_on_demand():
    """Test deploy in blocks on demand mode"""
    latest_block = gateway_call("get_block", blockNumber="latest")
    genesis_block_number = latest_block["block_number"]

    # Deploy and invoke
    deploy_info = deploy(CONTRACT_PATH, inputs=["0"])
    invoke(
        calls=[(deploy_info["address"], "increase_balance", [10, 20])],
        account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
    )

    latest_block = gateway_call("get_block", blockNumber="latest")
    block_number_after_deploy_and_invoke = latest_block["block_number"]

    requests.post(f"{APP_URL}/create_block_on_demand")
    latest_block = gateway_call("get_block", blockNumber="latest")
    block_number_after_block_on_demand_call = latest_block["block_number"]

    assert genesis_block_number == 0
    assert block_number_after_deploy_and_invoke == 0
    assert block_number_after_block_on_demand_call == 1
    assert len(latest_block["transactions"]) == 2
    assert latest_block["transactions"][0]["class_hash"] == CONTRAC_HASH_CLASS
