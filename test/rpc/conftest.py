"""
Fixtures for RPC tests
"""

from __future__ import annotations

import json
import typing

from starkware.starknet.services.api.contract_class import ContractClass
from starkware.starknet.services.api.gateway.transaction import Transaction, Deploy

import pytest
from starknet_devnet.blueprints.rpc.structures.types import (
    BlockNumberDict,
    BlockHashDict,
    Felt,
)
from ..util import load_file_content
from ..shared import SUPPORTED_RPC_TX_VERSION
from .rpc_utils import (
    gateway_call,
    get_block_with_transaction,
    pad_zero,
    add_transaction,
)

DEPLOY_CONTENT = load_file_content("deploy_rpc.json")
INVOKE_CONTENT = load_file_content("invoke_rpc.json")
INVOKE_CONTENT_V1 = load_file_content("invoke_rpc_v1.json")
DECLARE_CONTENT = load_file_content("declare.json")


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
    return pad_zero(class_hash)


@pytest.fixture(name="deploy_info")
def fixture_deploy_info() -> dict:
    """
    Deploy a contract on devnet and return deployment info dict
    """
    return add_transaction(json.loads(DEPLOY_CONTENT))


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


@pytest.fixture(name="invoke_content_v1")
def fixture_invoke_content_v1() -> dict:
    """
    Invoke content v1 JSON object
    """
    return json.loads(INVOKE_CONTENT_V1)


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


@pytest.fixture(name="block_id")
def fixture_block_id(gateway_block, request) -> dict:
    """
    BlockId of gateway_block depending on type in request
    """
    block_id_map = {
        "hash": BlockNumberDict(block_number=gateway_block["block_number"]),
        "number": BlockHashDict(block_hash=pad_zero(gateway_block["block_hash"])),
        "tag": "latest",
    }
    return block_id_map[request.param]


@pytest.fixture(name="rpc_invoke_tx_common")
def fixture_rpc_invoke_tx_common() -> dict:
    """
    Common fields on RpcInvokeTransaction
    """
    return {
        # It is not verified and might be removed in next RPC version
        "transaction_hash": "0x00",
        "max_fee": "0x00",
        "version": hex(SUPPORTED_RPC_TX_VERSION),
        "signature": [],
        "nonce": None,
        "type": "INVOKE",
    }
