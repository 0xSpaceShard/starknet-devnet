"""
Fixtures for RPC tests
"""

from __future__ import annotations

import json
import typing

from test.rpc.rpc_utils import (
    gateway_call,
    get_block_with_transaction,
    add_transaction,
    get_latest_block,
)
from test.util import load_file_content

import pytest
from starkware.starknet.services.api.contract_class import ContractClass
from starkware.starknet.services.api.gateway.transaction import Transaction, Deploy

from starknet_devnet.blueprints.rpc.utils import rpc_felt
from starknet_devnet.blueprints.rpc.structures.types import (
    BlockNumberDict,
    BlockHashDict,
    Felt,
)

DEPLOY_CONTENT = load_file_content("deploy_rpc.json")
INVOKE_CONTENT = load_file_content("invoke_rpc.json")
DECLARE_CONTENT = load_file_content("declare.json")
DECLARE_CONTENT = load_file_content("declare_rpc.json")


@pytest.fixture(name="contract_class")
def fixture_contract_class() -> ContractClass:
    """
    Make ContractDefinition from deployment transaction used in tests
    """
    transaction: Deploy = typing.cast(Deploy, Transaction.loads(DEPLOY_CONTENT))
    return transaction.contract_definition


@pytest.fixture(name="class_hash")
def fixture_class_hash(deploy_info) -> Felt:
    """
    Class hash of deployed contract
    """
    class_hash = gateway_call(
        "get_class_hash_at", contractAddress=deploy_info["address"]
    )
    return rpc_felt(class_hash)


@pytest.fixture(name="deploy_info")
def fixture_deploy_info() -> dict:
    """
    Deploy a contract on devnet and return deployment info dict
    """
    deploy_tx = json.loads(DEPLOY_CONTENT)
    deploy_info = add_transaction(deploy_tx)
    return {**deploy_info, **deploy_tx}


@pytest.fixture(name="invoke_info")
def fixture_invoke_info() -> dict:
    """
    Make an invoke transaction on devnet and return invoke info dict
    """
    invoke_tx = json.loads(INVOKE_CONTENT)
    invoke_info = add_transaction(invoke_tx)
    return {**invoke_info, **invoke_tx}


@pytest.fixture(name="declare_info")
def fixture_declare_info() -> dict:
    """
    Make a declare transaction on devnet and return declare info dict
    """
    declare_tx = json.loads(DECLARE_CONTENT)
    declare_info = add_transaction(declare_tx)
    return {**declare_info, **declare_tx}


@pytest.fixture(name="invoke_content")
def fixture_invoke_content() -> dict:
    """
    Invoke content JSON object
    """
    return json.loads(INVOKE_CONTENT)


@pytest.fixture(name="deploy_content")
def fixture_deploy_content() -> dict:
    """
    Deploy content JSON object
    """
    return json.loads(DEPLOY_CONTENT)


@pytest.fixture(name="declare_content")
def fixture_declare_content() -> dict:
    """
    Declare content JSON object
    """
    return json.loads(DECLARE_CONTENT)


@pytest.fixture(name="gateway_block")
def fixture_gateway_block(deploy_info) -> dict:
    """
    Block with Deploy transaction
    """
    return get_block_with_transaction(deploy_info["transaction_hash"])


@pytest.fixture(name="latest_block")
def fixture_latest_block() -> dict:
    """
    Latest block
    """
    return get_latest_block()


def _block_to_block_id(block: dict, key: str) -> dict:
    block_id_map = {
        "number": BlockNumberDict(block_number=int(block["block_number"])),
        "hash": BlockHashDict(block_hash=rpc_felt(block["block_hash"])),
        "tag": "latest",
        "tag_pending": "pending",
    }
    return block_id_map[key]


@pytest.fixture(name="block_id", params=["hash", "number", "tag"])
def fixture_block_id(gateway_block, request) -> dict:
    """
    BlockId of gateway_block depending on type in request
    """
    return _block_to_block_id(gateway_block, request.param)


@pytest.fixture(name="latest_block_id", params=["hash", "number", "tag", "tag_pending"])
def fixture_latest_block_id(latest_block, request) -> dict:
    """
    Parametrized BlockId of latest gateway_block
    """
    return _block_to_block_id(latest_block, request.param)
