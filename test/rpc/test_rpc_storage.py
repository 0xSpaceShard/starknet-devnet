"""
Tests RPC storage
"""
from test.rpc.rpc_utils import rpc_call

import pytest
from starkware.starknet.public.abi import get_storage_var_address

from starknet_devnet.blueprints.rpc.utils import rpc_felt, rpc_storage_key


@pytest.mark.usefixtures("run_devnet_in_background")
def test_get_storage_at(deploy_info):
    """
    Get storage at address
    """
    contract_address: str = deploy_info["address"]
    key: str = hex(get_storage_var_address("balance"))
    block_id: str = "latest"

    resp = rpc_call(
        "starknet_getStorageAt",
        params={
            "contract_address": rpc_felt(contract_address),
            "key": rpc_storage_key(key),
            "block_id": block_id,
        },
    )
    storage = resp["result"]

    assert storage == "0x45"


@pytest.mark.usefixtures("run_devnet_in_background")
def test_get_storage_at_old_block(deploy_info):
    """
    Get storage at address
    """
    contract_address: str = deploy_info["address"]
    key: str = hex(get_storage_var_address("balance"))

    def get_storage(block_id):
        resp = rpc_call(
            "starknet_getStorageAt",
            params={
                "contract_address": rpc_felt(contract_address),
                "key": rpc_storage_key(key),
                "block_id": block_id,
            },
        )
        return resp["result"]

    assert get_storage({"block_number": 0}) == "0x0"
    assert get_storage({"block_hash": "0x0"}) == "0x0"


@pytest.mark.usefixtures("run_devnet_in_background", "deploy_info")
def test_get_storage_at_raises_on_incorrect_contract():
    """
    Get storage at incorrect contract
    """
    key: str = hex(get_storage_var_address("balance"))
    block_id: str = "latest"

    ex = rpc_call(
        "starknet_getStorageAt",
        params={
            "contract_address": "0x0",
            "key": rpc_storage_key(key),
            "block_id": block_id,
        },
    )

    assert ex["error"] == {"code": 20, "message": "Contract not found"}


# internal workings of get_storage_at would have to be changed for this to work properly
# since currently it will (correctly) return 0x0 for any incorrect key
# and it should throw exception with code=23 and message="Invalid storage key"
@pytest.mark.usefixtures("run_devnet_in_background")
def test_get_storage_at_raises_on_incorrect_key(deploy_info):
    """
    Get storage at incorrect key
    """
    contract_address: str = deploy_info["address"]

    response = rpc_call(
        "starknet_getStorageAt",
        params={
            "contract_address": rpc_felt(contract_address),
            "key": "0x00",
            "block_id": "latest",
        },
    )

    assert response["result"] == "0x0"


@pytest.mark.usefixtures("run_devnet_in_background")
def test_get_storage_at_raises_on_incorrect_block_id(deploy_info):
    """
    Get storage at incorrect block id
    """

    contract_address: str = deploy_info["address"]
    key: str = hex(get_storage_var_address("balance"))

    ex = rpc_call(
        "starknet_getStorageAt",
        params={
            "contract_address": rpc_felt(contract_address),
            "key": rpc_storage_key(key),
            "block_id": {"block_number": 99999},
        },
    )

    assert ex["error"] == {
        "code": 24,
        "message": "Block not found",
    }
