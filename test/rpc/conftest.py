"""
Fixtures for RPC tests
"""

from __future__ import annotations

import json
from test.account import deploy
from test.rpc.rpc_utils import (
    add_transaction,
    gateway_call,
    get_block_with_transaction,
    get_latest_block,
)
from test.test_account import SALT
from test.util import load_file_content, mint
from typing import Tuple

import pytest
from starkware.crypto.signature.signature import private_to_stark_key
from starkware.starknet.business_logic.transaction.objects import InternalDeployAccount
from starkware.starknet.core.os.contract_class.deprecated_class_hash import (
    compute_deprecated_class_hash,
)
from starkware.starknet.definitions.general_config import DEFAULT_CHAIN_ID
from starkware.starknet.services.api.contract_class.contract_class import (
    CompiledClassBase,
)
from starkware.starknet.services.api.feeder_gateway.response_objects import (
    DeployAccountSpecificInfo,
)
from starkware.starknet.services.api.gateway.transaction import DeployAccount
from starkware.starknet.third_party.open_zeppelin.starknet_contracts import (
    account_contract as oz_account_class,
)
from starkware.starknet.wallets.open_zeppelin import sign_deploy_account_tx

from starknet_devnet.blueprints.rpc.structures.payloads import (
    RpcDeployAccountTransaction,
    rpc_deploy_account_transaction,
)
from starknet_devnet.blueprints.rpc.structures.types import (
    BlockHashDict,
    BlockNumberDict,
    Felt,
)
from starknet_devnet.blueprints.rpc.utils import rpc_felt
from starknet_devnet.chargeable_account import ChargeableAccount
from starknet_devnet.constants import SUPPORTED_RPC_TX_VERSION
from starknet_devnet.general_config import DEFAULT_GENERAL_CONFIG

DECLARE_CONTENT = load_file_content("declare_rpc.json")

PRIVATE_KEY = 123456789987654321
PUBLIC_KEY = private_to_stark_key(PRIVATE_KEY)


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
    Deploy a contract using chargeable account. Return deployment info dict.
    """
    declare_tx = json.loads(DECLARE_CONTENT)
    declare_info = add_transaction(declare_tx)

    deploy_info = deploy(
        class_hash=declare_info["class_hash"],
        account_address=hex(ChargeableAccount.ADDRESS),
        private_key=ChargeableAccount.PRIVATE_KEY,
        inputs=[69],
        salt="0x2",
        max_fee=int(1e18),
    )

    return deploy_info


@pytest.fixture(name="declare_info")
def fixture_declare_info() -> dict:
    """
    Make a declare transaction on devnet and return declare info dict
    """
    declare_tx = json.loads(DECLARE_CONTENT)
    declare_info = add_transaction(declare_tx)
    return {**declare_info, **declare_tx}


@pytest.fixture(name="deploy_account_info")
def fixture_deploy_account_info() -> dict:
    """
    Make a deploy account transaction on devnet and return deploy account info dict
    """
    deploy_account_tx, address = prepare_deploy_account_tx(
        PRIVATE_KEY, PUBLIC_KEY, int(SALT, 16), oz_account_class
    )
    mint(hex(address), amount=int(1e18))
    deploy_account_json = deploy_account_tx.dump()
    deploy_account_json["type"] = "DEPLOY_ACCOUNT"
    declare_info = add_transaction(deploy_account_json)
    return declare_info


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
    return get_block_with_transaction(deploy_info["tx_hash"])


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


@pytest.fixture(name="block_id", params=["hash", "number", "tag", "tag_pending"])
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


@pytest.fixture(
    name="deploy_account_details",
)
def fixture_deploy_account_details() -> dict:
    """Deploy account transaction details"""
    return {
        "private_key": 0x6F9E0F15B20753CE2E2B740B182099C4ADF765D0C5A5B75C1AF3327358FBF2E,
        "public_key": 0x7707342F75277F32F1A0AD532E1A12016B36A3967332D31F915C889678B3DB6,
        "account_salt": 0x75B567ECB69C6D032982FA32C8F52D2F00DB50C5DE2C93EDDA70DE9B5109F8F,
        "contract_class": oz_account_class,
    }


def prepare_deploy_account_tx(
    private_key: int,
    public_key: int,
    account_salt: int,
    contract_class: CompiledClassBase,
) -> Tuple[DeployAccount, int]:
    """Return (signed deploy account tx, address)"""
    account_address, deploy_account_tx = sign_deploy_account_tx(
        private_key=private_key,
        public_key=public_key,
        class_hash=compute_deprecated_class_hash(contract_class),
        salt=account_salt,
        max_fee=int(1e18),
        version=SUPPORTED_RPC_TX_VERSION,
        chain_id=DEFAULT_CHAIN_ID,
        nonce=0,
    )

    return deploy_account_tx, account_address


def rpc_deploy_account_from_gateway(
    deploy_account_tx: DeployAccount,
) -> RpcDeployAccountTransaction:
    """Convert DeployAccount to RpcDeployAccountTransaction"""
    internal_deploy_account = InternalDeployAccount.from_external(
        external_tx=deploy_account_tx, general_config=DEFAULT_GENERAL_CONFIG
    )
    deploy_account_specific_info = (
        DeployAccountSpecificInfo.from_internal_deploy_account(internal_deploy_account)
    )
    rpc_deploy_account_tx = rpc_deploy_account_transaction(deploy_account_specific_info)
    return rpc_deploy_account_tx
