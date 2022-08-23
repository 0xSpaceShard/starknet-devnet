"""
Tests RPC miscellaneous
"""

from __future__ import annotations

import pytest
from starkware.starknet.public.abi import get_storage_var_address
from starkware.starknet.core.os.class_hash import compute_class_hash

from starknet_devnet.blueprints.rpc.structures.types import BlockHashDict
from starknet_devnet.general_config import DEFAULT_GENERAL_CONFIG

from .rpc_utils import rpc_call, gateway_call, get_block_with_transaction, pad_zero


# pylint: disable=too-many-locals
@pytest.mark.usefixtures("run_devnet_in_background")
def test_get_state_update(deploy_info, invoke_info, contract_class):
    """
    Get state update for the block
    """
    block_with_deploy = get_block_with_transaction(deploy_info["transaction_hash"])
    block_with_invoke = get_block_with_transaction(invoke_info["transaction_hash"])

    contract_address: str = deploy_info["address"]
    block_with_deploy_hash: str = pad_zero(block_with_deploy["block_hash"])
    block_with_invoke_hash: str = pad_zero(block_with_invoke["block_hash"])
    block_id_deploy = BlockHashDict(block_hash=block_with_deploy_hash)
    block_id_invoke = BlockHashDict(block_hash=block_with_invoke_hash)
    class_hash = pad_zero(hex(compute_class_hash(contract_class)))

    storage = gateway_call("get_storage_at", contractAddress=contract_address, key=get_storage_var_address("balance"))

    new_root_deploy = "0x0" + gateway_call("get_state_update", blockHash=block_with_deploy_hash)["new_root"].lstrip("0")
    new_root_invoke = "0x0" + gateway_call("get_state_update", blockHash=block_with_invoke_hash)["new_root"].lstrip("0")

    resp = rpc_call(
        "starknet_getStateUpdate", params={
            "block_id": block_id_deploy
        }
    )
    state_update = resp["result"]

    assert state_update["block_hash"] == block_with_deploy_hash
    assert state_update["new_root"] == new_root_deploy
    assert "old_root" in state_update
    assert isinstance(state_update["old_root"], str)
    assert state_update["state_diff"] == {
        "storage_diffs": [],
        "deployed_contracts": [
            {
                "address": pad_zero(contract_address),
                "class_hash": class_hash,
            }
        ],
        "declared_contracts": [
            {
                "class_hash": class_hash,
            }
        ],
        "nonces": [],
    }

    resp = rpc_call(
        "starknet_getStateUpdate", params={
            "block_id": block_id_invoke
        }
    )
    state_update = resp["result"]

    assert state_update["block_hash"] == block_with_invoke_hash
    assert state_update["new_root"] == new_root_invoke
    assert "old_root" in state_update
    assert isinstance(state_update["old_root"], str)
    assert state_update["state_diff"] == {
        "storage_diffs": [
            {
                "address": contract_address,
                "key": pad_zero(hex(get_storage_var_address("balance"))),
                "value": pad_zero(storage),
            }
        ],
        "deployed_contracts": [],
        "declared_contracts": [],
        "nonces": [],
    }


@pytest.mark.usefixtures("run_devnet_in_background")
def test_chain_id():
    """
    Test chain id
    """
    chain_id = DEFAULT_GENERAL_CONFIG.chain_id.value

    resp = rpc_call("starknet_chainId", params={})
    rpc_chain_id = resp["result"]

    assert isinstance(rpc_chain_id, str)
    assert rpc_chain_id == hex(chain_id)


@pytest.mark.usefixtures("run_devnet_in_background")
def test_syncing():
    """
    Test syncing
    """
    resp = rpc_call("starknet_syncing", params={})
    syncing = resp["result"]

    assert isinstance(syncing, bool)
    assert syncing is False
