"""
Test blocks on demand mode.
"""

import requests
from test.rpc.rpc_utils import gateway_call

from .settings import APP_URL
from .shared import (
    CONTRACT_PATH,
    PREDEPLOY_ACCOUNT_CLI_ARGS,
)
from .util import (
    deploy,
    devnet_in_background,
)

@devnet_in_background(*[*PREDEPLOY_ACCOUNT_CLI_ARGS,"--blocks-on-demand",])
def test_blocks_on_demand():
    """Test deploy in blocks on demand mode"""
    latest_block = gateway_call("get_block", blockNumber="latest")
    block_number_before_deploy = latest_block["block_number"]
    deploy(contract=CONTRACT_PATH)
    latest_block = gateway_call("get_block", blockNumber="latest")
    block_number_after_deploy = latest_block["block_number"]

    requests.post(f"{APP_URL}/create_block_on_demand")
    latest_block = gateway_call("get_block", blockNumber="latest")
    block_number_after_block_on_demand_call = latest_block["block_number"]

    assert block_number_before_deploy == 0
    assert block_number_after_deploy == 0
    assert block_number_after_block_on_demand_call == 1
