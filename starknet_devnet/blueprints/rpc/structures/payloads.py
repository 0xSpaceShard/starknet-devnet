"""
RPC payload structures
"""

from __future__ import annotations

from typing import Callable, Union, List, Optional

from starkware.starknet.definitions.general_config import StarknetGeneralConfig
from starkware.starknet.services.api.contract_class import ContractClass
from starkware.starknet.services.api.feeder_gateway.response_objects import (
    StarknetBlock,
    InvokeSpecificInfo,
    DeploySpecificInfo,
    TransactionSpecificInfo,
    TransactionType,
    BlockStateUpdate,
    DeclareSpecificInfo,
)
from starkware.starknet.services.api.gateway.transaction import InvokeFunction
from starkware.starknet.services.api.gateway.transaction_utils import compress_program
from typing_extensions import TypedDict

from starknet_devnet.blueprints.rpc.utils import rpc_root, rpc_felt
from starknet_devnet.blueprints.rpc.structures.types import (
    RpcBlockStatus,
    BlockHash,
    BlockNumber,
    Felt,
    rpc_block_status,
    TxnHash,
    Address,
    NumAsHex,
    TxnType,
    rpc_txn_type,
)
from starknet_devnet.state import state


class RpcBlock(TypedDict):
    """
    TypeDict for rpc block
    """

    status: RpcBlockStatus
    block_hash: BlockHash
    parent_hash: BlockHash
    block_number: BlockNumber
    new_root: Felt
    timestamp: int
    sequencer_address: Felt
    transactions: Union[List[str], List[RpcTransaction]]


async def rpc_block(
    block: StarknetBlock, tx_type: Optional[str] = "TXN_HASH"
) -> RpcBlock:
    """
    Convert gateway block to rpc block
    """

    async def transactions() -> List[RpcTransaction]:
        # pylint: disable=no-member
        return [rpc_transaction(tx) for tx in block.transactions]

    async def transaction_hashes() -> List[str]:
        return [tx["transaction_hash"] for tx in await transactions()]

    def new_root() -> Felt:
        # pylint: disable=no-member
        return rpc_root(block.state_root.hex())

    def config() -> StarknetGeneralConfig:
        devnet_state = state.starknet_wrapper.get_state()
        _config = devnet_state.general_config
        return _config

    mapping: dict[str, Callable] = {
        "TXN_HASH": transaction_hashes,
        "FULL_TXNS": transactions,
    }
    transactions: list = await mapping[tx_type]()

    block: RpcBlock = {
        "status": rpc_block_status(block.status.name),
        "block_hash": rpc_felt(block.block_hash),
        "parent_hash": rpc_felt(block.parent_block_hash or 0),
        "block_number": block.block_number if block.block_number is not None else 0,
        "new_root": new_root(),
        "timestamp": block.timestamp,
        "sequencer_address": rpc_felt(config().sequencer_address),
        "transactions": transactions,
    }
    return block


class RpcInvokeTransaction(TypedDict):
    """
    TypedDict for rpc invoke transaction
    """

    contract_address: Address
    entry_point_selector: Felt
    calldata: List[Felt]
    # Common
    transaction_hash: TxnHash
    max_fee: Felt
    version: NumAsHex
    signature: List[Felt]
    nonce: Felt
    type: TxnType


class RpcDeclareTransaction(TypedDict):
    """
    TypedDict for rpc declare transaction
    """

    class_hash: Felt
    sender_address: Address
    # Common
    transaction_hash: TxnHash
    max_fee: Felt
    version: NumAsHex
    signature: List[Felt]
    nonce: Felt
    type: TxnType


class RpcDeployTransaction(TypedDict):
    """
    TypedDict for rpc deploy transaction
    """

    transaction_hash: TxnHash
    class_hash: Felt
    version: NumAsHex
    type: TxnType
    contract_address: Felt
    contract_address_salt: Felt
    constructor_calldata: List[Felt]


RpcTransaction = Union[
    RpcInvokeTransaction, RpcDeclareTransaction, RpcDeployTransaction
]


def rpc_transaction(transaction: TransactionSpecificInfo) -> RpcTransaction:
    """
    Convert gateway transaction to rpc transaction
    """
    tx_mapping = {
        TransactionType.DEPLOY: rpc_deploy_transaction,
        TransactionType.INVOKE_FUNCTION: rpc_invoke_transaction,
        TransactionType.DECLARE: rpc_declare_transaction,
    }
    return tx_mapping[transaction.tx_type](transaction)


class FunctionCall(TypedDict):
    """
    TypedDict for rpc function call
    """

    contract_address: Address
    entry_point_selector: Felt
    calldata: List[Felt]


def rpc_invoke_transaction(transaction: InvokeSpecificInfo) -> RpcInvokeTransaction:
    """
    Convert gateway invoke transaction to rpc format
    """
    txn: RpcInvokeTransaction = {
        "contract_address": rpc_felt(transaction.contract_address),
        "entry_point_selector": rpc_felt(transaction.entry_point_selector),
        "calldata": [rpc_felt(data) for data in transaction.calldata],
        "transaction_hash": rpc_felt(transaction.transaction_hash),
        "max_fee": rpc_felt(transaction.max_fee),
        "version": hex(0x0),
        "signature": [rpc_felt(value) for value in transaction.signature],
        "nonce": rpc_felt(0),
        "type": rpc_txn_type(transaction.tx_type.name),
    }
    return txn


def rpc_declare_transaction(transaction: DeclareSpecificInfo) -> RpcDeclareTransaction:
    """
    Convert gateway declare transaction to rpc format
    """
    txn: RpcDeclareTransaction = {
        "class_hash": rpc_felt(transaction.class_hash),
        "sender_address": rpc_felt(transaction.sender_address),
        "transaction_hash": rpc_felt(transaction.transaction_hash),
        "max_fee": rpc_felt(transaction.max_fee),
        "version": hex(transaction.version),
        "signature": [rpc_felt(value) for value in transaction.signature],
        "nonce": rpc_felt(transaction.nonce),
        "type": rpc_txn_type(transaction.tx_type.name),
    }
    return txn


def rpc_deploy_transaction(transaction: DeploySpecificInfo) -> RpcDeployTransaction:
    """
    Convert gateway deploy transaction to rpc format
    """
    txn: RpcDeployTransaction = {
        "transaction_hash": rpc_felt(transaction.transaction_hash),
        "class_hash": rpc_felt(transaction.class_hash),
        "version": hex(0x0),
        "type": rpc_txn_type(transaction.tx_type.name),
        "contract_address": rpc_felt(transaction.contract_address),
        "contract_address_salt": rpc_felt(transaction.contract_address_salt),
        "constructor_calldata": [
            rpc_felt(data) for data in transaction.constructor_calldata
        ],
    }
    return txn


class RpcFeeEstimate(TypedDict):
    """
    Fee estimate TypedDict for rpc
    """

    gas_consumed: NumAsHex
    gas_price: NumAsHex
    overall_fee: NumAsHex


def rpc_fee_estimate(fee_estimate: dict) -> dict:
    """
    Convert gateway estimate_fee response to rpc_fee_estimate
    """
    result: RpcFeeEstimate = {
        "gas_consumed": hex(fee_estimate["gas_usage"]),
        "gas_price": hex(fee_estimate["gas_price"]),
        "overall_fee": hex(fee_estimate["overall_fee"]),
    }
    return result


def make_invoke_function(request_body: dict) -> InvokeFunction:
    """
    Convert RPC request to internal InvokeFunction
    """
    return InvokeFunction(
        contract_address=int(request_body["contract_address"], 16),
        entry_point_selector=int(request_body["entry_point_selector"], 16),
        calldata=[int(data, 16) for data in request_body["calldata"]],
        max_fee=int(request_body["max_fee"], 16) if "max_fee" in request_body else 0,
        version=int(request_body["version"], 16) if "version" in request_body else 0,
        signature=[int(data, 16) for data in request_body.get("signature", [])],
    )


class EntryPoint(TypedDict):
    """
    TypedDict for rpc contract class entry point
    """

    offset: NumAsHex
    selector: Felt


class EntryPoints(TypedDict):
    """
    TypedDict for rpc contract class entry points
    """

    CONSTRUCTOR: List[EntryPoint]
    EXTERNAL: List[EntryPoint]
    L1_HANDLER: List[EntryPoint]


class RpcContractClass(TypedDict):
    """
    TypedDict for rpc contract class
    """

    program: str
    entry_points_by_type: EntryPoints


def rpc_contract_class(contract_class: ContractClass) -> RpcContractClass:
    """
    Convert gateway contract class to rpc contract class
    """

    def program() -> str:
        _program = contract_class.program.Schema().dump(contract_class.program)
        return compress_program(_program)

    def entry_points_by_type() -> EntryPoints:
        _entry_points: EntryPoints = {
            "CONSTRUCTOR": [],
            "EXTERNAL": [],
            "L1_HANDLER": [],
        }
        for typ, entry_points in contract_class.entry_points_by_type.items():
            for entry_point in entry_points:
                _entry_point: EntryPoint = {
                    "selector": rpc_felt(entry_point.selector),
                    "offset": hex(entry_point.offset),
                }
                _entry_points[typ.name].append(_entry_point)
        return _entry_points

    _contract_class: RpcContractClass = {
        "program": program(),
        "entry_points_by_type": entry_points_by_type(),
    }
    return _contract_class


class RpcStorageDiff(TypedDict):
    """
    TypedDict for rpc storage diff
    """

    address: Felt
    key: Felt
    value: Felt


class RpcDeclaredContractDiff(TypedDict):
    """
    TypedDict for rpc declared contract diff
    """

    class_hash: Felt


class RpcDeployedContractDiff(TypedDict):
    """
    TypedDict for rpc deployed contract diff
    """

    address: Felt
    class_hash: Felt


class RpcNonceDiff(TypedDict):
    """
    TypedDict for rpc nonce diff
    """

    contract_address: Address
    nonce: Felt


class RpcStateDiff(TypedDict):
    """
    TypedDict for rpc state diff
    """

    storage_diffs: List[RpcStorageDiff]
    declared_contracts: List[RpcDeclaredContractDiff]
    deployed_contracts: List[RpcDeployedContractDiff]
    nonces: List[RpcNonceDiff]


class RpcStateUpdate(TypedDict):
    """
    TypedDict for rpc state update
    """

    block_hash: BlockHash
    new_root: Felt
    old_root: Felt
    state_diff: RpcStateDiff


def rpc_state_update(state_update: BlockStateUpdate) -> RpcStateUpdate:
    """
    Convert gateway state update to rpc state update
    """

    def storage_diffs() -> List[RpcStorageDiff]:
        _storage_diffs = []
        for address, diffs in state_update.state_diff.storage_diffs.items():
            for diff in diffs:
                _diff: RpcStorageDiff = {
                    "address": rpc_felt(address),
                    "key": rpc_felt(diff.key),
                    "value": rpc_felt(diff.value),
                }
                _storage_diffs.append(_diff)
        return _storage_diffs

    def declared_contracts() -> List[RpcDeclaredContractDiff]:
        _contracts = []
        for contract in state_update.state_diff.declared_contracts:
            diff: RpcDeclaredContractDiff = {"class_hash": rpc_felt(contract)}
            _contracts.append(diff)
        return _contracts

    def deployed_contracts() -> List[RpcDeployedContractDiff]:
        _contracts = []
        for contract in state_update.state_diff.deployed_contracts:
            diff: RpcDeployedContractDiff = {
                "address": rpc_felt(contract.address),
                "class_hash": rpc_felt(contract.class_hash),
            }
            _contracts.append(diff)
        return _contracts

    rpc_state: RpcStateUpdate = {
        "block_hash": rpc_felt(state_update.block_hash),
        "new_root": rpc_root(state_update.new_root.hex()),
        "old_root": rpc_root(state_update.old_root.hex()),
        "state_diff": {
            "storage_diffs": storage_diffs(),
            "declared_contracts": declared_contracts(),
            "deployed_contracts": deployed_contracts(),
            "nonces": [],
        },
    }
    return rpc_state
