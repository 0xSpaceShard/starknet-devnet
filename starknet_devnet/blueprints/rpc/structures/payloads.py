"""
RPC payload structures
"""

from __future__ import annotations

from typing import Callable, Dict, List, Optional, Union

from marshmallow.exceptions import MarshmallowError
from starkware.starknet.definitions.general_config import StarknetGeneralConfig
from starkware.starknet.public.abi import AbiEntryType
from starkware.starknet.services.api.contract_class.contract_class import (
    ContractClass,
    ContractEntryPoint,
    DeprecatedCompiledClass,
    EntryPointType,
)
from starkware.starknet.services.api.feeder_gateway.request_objects import CallFunction
from starkware.starknet.services.api.feeder_gateway.response_objects import (
    BlockStateUpdate,
    DeclareSpecificInfo,
    DeployAccountSpecificInfo,
    DeploySpecificInfo,
    FeeEstimationInfo,
    InvokeSpecificInfo,
    L1HandlerSpecificInfo,
    StarknetBlock,
    TransactionSpecificInfo,
    TransactionType,
)
from starkware.starknet.services.api.gateway.transaction import (
    Declare,
    DeployAccount,
    DeprecatedDeclare,
    InvokeFunction,
)
from starkware.starknet.services.api.gateway.transaction_utils import (
    compress_program,
    decompress_program,
)
from starkware.starkware_utils.error_handling import StarkException
from typing_extensions import Literal, TypedDict

from starknet_devnet.blueprints.rpc.structures.types import (
    Address,
    BlockHash,
    BlockNumber,
    Felt,
    NumAsHex,
    RpcBlockStatus,
    RpcError,
    RpcTxnType,
    Signature,
    TxnHash,
    rpc_block_status,
    rpc_txn_type,
)
from starknet_devnet.blueprints.rpc.utils import rpc_felt, rpc_root
from starknet_devnet.constants import (
    DEPRECATED_RPC_DECLARE_TX_VERSION,
    LEGACY_RPC_TX_VERSION,
    SUPPORTED_RPC_DECLARE_TX_VERSION,
)
from starknet_devnet.state import state


class RpcBlock(TypedDict):
    """TypedDict for rpc block"""

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


class RpcBroadcastedTxnCommon(TypedDict):
    """TypedDict for RpcBroadcastedTxnCommon"""

    type: RpcTxnType
    max_fee: Felt
    version: NumAsHex
    signature: Signature
    nonce: Felt


class RpcBroadcastedInvokeTxnV0(RpcBroadcastedTxnCommon):
    """TypedDict for RpcBroadcastedInvokeTxnV0"""

    contract_address: Address
    entry_point_selector: Felt
    calldata: List[Felt]


class RpcBroadcastedInvokeTxnV1(RpcBroadcastedTxnCommon):
    """TypedDict for RpcBroadcastedInvokeTxnV1"""

    sender_address: Address
    calldata: List[Felt]


RpcBroadcastedInvokeTxn = Union[RpcBroadcastedInvokeTxnV0, RpcBroadcastedInvokeTxnV1]


class RpcBroadcastedDeclareTxnV1(RpcBroadcastedTxnCommon):
    """TypedDict for RpcBroadcastDeclareTxnV1"""

    contract_class: RpcDeprecatedContractClass
    sender_address: Address


class RpcBroadcastedDeclareTxnV2(RpcBroadcastedTxnCommon):
    """TypedDict for RpcBroadcastedDeclareTxnV2"""

    contract_class: RpcContractClass
    sender_address: Address
    compiled_class_hash: Felt


RpcBroadcastedDeclareTxn = Union[RpcBroadcastedDeclareTxnV1, RpcBroadcastedDeclareTxnV2]


class RpcBroadcastedDeployAccountTxn(RpcBroadcastedTxnCommon):
    """TypedDict for BroadcastedDeployAccountTxn"""

    contract_address_salt: Felt
    constructor_calldata: List[Felt]
    class_hash: Felt


# rpc transaction's representation when it's sent to the sequencer (but not yet in a block)
RpcBroadcastedTxn = Union[
    RpcBroadcastedDeclareTxn,
    RpcBroadcastedInvokeTxn,
    RpcBroadcastedDeployAccountTxn,
]


class RpcTransactionCommon(RpcBroadcastedTxnCommon):
    """TypedDict for RpcTransactionCommon"""

    transaction_hash: TxnHash


class RpcInvokeTransactionV0(RpcTransactionCommon):
    """TypedDict for rpc invoke transaction version 0"""

    contract_address: Address
    entry_point_selector: Felt
    calldata: List[Felt]


class RpcInvokeTransactionV1(RpcTransactionCommon):
    """TypedDict for rpc invoke transaction version 1"""

    sender_address: Address
    calldata: List[Felt]


class RpcL1HandlerTransaction(TypedDict):
    """TypedDict for rpc L1 -> L2 message transaction"""

    contract_address: Address
    entry_point_selector: Felt
    calldata: List[Felt]
    transaction_hash: TxnHash
    version: NumAsHex
    type: RpcTxnType
    nonce: Felt


class RpcDeclareTransaction(RpcTransactionCommon):
    """TypedDict for rpc declare transaction"""

    class_hash: Felt
    sender_address: Address


class RpcDeployTransaction(TypedDict):
    """TypedDict for rpc deploy transaction"""

    transaction_hash: TxnHash
    class_hash: Felt
    version: NumAsHex
    type: RpcTxnType
    contract_address_salt: Felt
    constructor_calldata: List[Felt]


class RpcDeployAccountTransaction(RpcTransactionCommon):
    """TypedDict for rpc deploy account transaction"""

    contract_address_salt: Felt
    constructor_calldata: List[Felt]
    class_hash: Felt


RpcTransaction = Union[
    RpcInvokeTransactionV0,
    RpcInvokeTransactionV1,
    RpcL1HandlerTransaction,
    RpcDeclareTransaction,
    RpcDeployTransaction,
    RpcDeployAccountTransaction,
]


def rpc_transaction(transaction: TransactionSpecificInfo) -> RpcTransaction:
    """Convert gateway transaction to rpc transaction"""
    tx_mapping = {
        TransactionType.DEPLOY: rpc_deploy_transaction,
        TransactionType.INVOKE_FUNCTION: rpc_invoke_transaction,
        TransactionType.DECLARE: rpc_declare_transaction,
        TransactionType.L1_HANDLER: rpc_l1_handler_transaction,
        TransactionType.DEPLOY_ACCOUNT: rpc_deploy_account_transaction,
    }
    return tx_mapping[transaction.tx_type](transaction)


class RpcFunctionCall(TypedDict):
    """TypedDict for rpc function call"""

    contract_address: Address
    entry_point_selector: Felt
    calldata: List[Felt]


def make_call_function(function_call: RpcFunctionCall) -> CallFunction:
    """
    Convert RPCFunctionCall to CallFunction
    """
    return CallFunction(
        contract_address=int(function_call["contract_address"], 16),
        entry_point_selector=int(function_call["entry_point_selector"], 16),
        calldata=[int(data, 16) for data in function_call["calldata"]],
    )


def rpc_invoke_transaction(
    transaction: InvokeSpecificInfo,
) -> Union[RpcInvokeTransactionV0, RpcInvokeTransactionV1]:
    """
    Convert gateway invoke transaction to rpc format
    """
    common_data = {
        "transaction_hash": rpc_felt(transaction.transaction_hash),
        "calldata": [rpc_felt(data) for data in transaction.calldata],
        "max_fee": rpc_felt(transaction.max_fee),
        "version": hex(transaction.version),
        "signature": [rpc_felt(value) for value in transaction.signature],
        "type": rpc_txn_type(transaction.tx_type.name),
        "nonce": rpc_felt(transaction.nonce or 0),
    }

    if transaction.version == LEGACY_RPC_TX_VERSION:
        txn: RpcInvokeTransactionV0 = {
            "contract_address": rpc_felt(transaction.sender_address),
            "entry_point_selector": rpc_felt(transaction.entry_point_selector),
            **common_data,
        }
    else:
        txn: RpcInvokeTransactionV1 = {
            "sender_address": rpc_felt(transaction.sender_address),
            **common_data,
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
        "version": hex(transaction.version),
        "type": rpc_txn_type(transaction.tx_type.name),
        "contract_address_salt": rpc_felt(transaction.contract_address_salt),
        "constructor_calldata": [
            rpc_felt(data) for data in transaction.constructor_calldata
        ],
    }
    return txn


def rpc_deploy_account_transaction(
    transaction: DeployAccountSpecificInfo,
) -> RpcDeployAccountTransaction:
    """
    Convert gateway deploy account transaction to rpc format
    """
    txn: RpcDeployAccountTransaction = {
        "contract_address_salt": rpc_felt(transaction.contract_address_salt),
        "constructor_calldata": [
            rpc_felt(data) for data in transaction.constructor_calldata
        ],
        "class_hash": rpc_felt(transaction.class_hash),
        "transaction_hash": rpc_felt(transaction.transaction_hash),
        "type": rpc_txn_type(transaction.tx_type.name),
        "max_fee": rpc_felt(transaction.max_fee),
        "version": hex(transaction.version),
        "signature": [rpc_felt(value) for value in transaction.signature],
        "nonce": rpc_felt(transaction.nonce),
    }
    return txn


def rpc_l1_handler_transaction(
    transaction: L1HandlerSpecificInfo,
) -> RpcL1HandlerTransaction:
    """
    Convert gateway l1_handler transaction to rpc format
    """
    txn: RpcL1HandlerTransaction = {
        "contract_address": rpc_felt(transaction.contract_address),
        "entry_point_selector": rpc_felt(transaction.entry_point_selector),
        "calldata": [rpc_felt(data) for data in transaction.calldata],
        "transaction_hash": rpc_felt(transaction.transaction_hash),
        "version": hex(transaction.version),
        "type": rpc_txn_type(transaction.tx_type.name),
        "nonce": rpc_felt(transaction.nonce),
    }
    return txn


class RpcFeeEstimate(TypedDict):
    """Fee estimate TypedDict for rpc"""

    gas_consumed: NumAsHex
    gas_price: NumAsHex
    overall_fee: NumAsHex


def rpc_fee_estimate(fee_estimates: List[FeeEstimationInfo]) -> list:
    """
    Convert gateway estimate_fee response to rpc_fee_estimate
    """

    result = [
        {
            "gas_consumed": hex(fee_estimate.gas_usage),
            "gas_price": hex(fee_estimate.gas_price),
            "overall_fee": hex(fee_estimate.overall_fee),
        }
        for fee_estimate in fee_estimates
    ]

    return result


def make_invoke_function(invoke_transaction: RpcBroadcastedInvokeTxn) -> InvokeFunction:
    """
    Convert RpcBroadcastedInvokeTxn to InvokeFunction
    """
    version = int(invoke_transaction["version"], 16)
    nonce = invoke_transaction.get("nonce")

    common_data = {
        "max_fee": int(invoke_transaction.get("max_fee", "0"), 16),
        "version": version,
        "signature": [
            int(data, 16) for data in invoke_transaction.get("signature", [])
        ],
        "nonce": int(nonce, 16) if version != LEGACY_RPC_TX_VERSION else None,
    }

    if version == LEGACY_RPC_TX_VERSION:
        invoke_function = InvokeFunction(
            sender_address=int(invoke_transaction["contract_address"], 16),
            entry_point_selector=int(invoke_transaction["entry_point_selector"], 16),
            calldata=[int(data, 16) for data in invoke_transaction.get("calldata", [])],
            **common_data,
        )
    else:
        invoke_function = InvokeFunction(
            sender_address=int(invoke_transaction["sender_address"], 16),
            calldata=[int(data, 16) for data in invoke_transaction.get("calldata", [])],
            **common_data,
        )

    return invoke_function


def make_declare_v1(
    declare_transaction: RpcBroadcastedDeclareTxnV1,
) -> DeprecatedDeclare:
    """
    Convert RpcBroadcastedDeclareTxnV1 to DeprecatedDeclare
    """
    contract_class = declare_transaction["contract_class"]
    if "abi" not in contract_class:
        contract_class["abi"] = []

    try:
        contract_class["program"] = decompress_program(contract_class["program"])
        contract_class = DeprecatedCompiledClass.load(contract_class)
    except (StarkException, TypeError, MarshmallowError) as ex:
        raise RpcError(code=50, message="Invalid contract class") from ex

    nonce = declare_transaction.get("nonce")
    declare_tx = DeprecatedDeclare(
        contract_class=contract_class,
        sender_address=int(declare_transaction["sender_address"], 16),
        nonce=int(nonce, 16) if nonce is not None else 0,
        version=int(declare_transaction["version"], 16),
        max_fee=int(declare_transaction["max_fee"], 16),
        signature=[int(sig, 16) for sig in declare_transaction["signature"]],
    )
    return declare_tx


def make_declare_v2(declare_transaction: RpcBroadcastedDeclareTxnV2) -> Declare:
    """Convert RpcBroadcastedDeclareTxnV2 to Declare"""
    nonce = declare_transaction.get("nonce")

    contract_class = declare_transaction["contract_class"]

    if "abi" not in contract_class:
        contract_class["abi"] = ""

    contract_class = ContractClass.load(contract_class)

    return Declare(
        contract_class=contract_class,
        compiled_class_hash=int(declare_transaction["compiled_class_hash"], 16),
        sender_address=int(declare_transaction["sender_address"], 16),
        nonce=int(nonce, 16) if nonce is not None else 0,
        version=int(declare_transaction["version"], 16),
        max_fee=int(declare_transaction["max_fee"], 16),
        signature=[int(sig, 16) for sig in declare_transaction["signature"]],
    )


def make_declare(
    declare_transaction: RpcBroadcastedDeclareTxn,
) -> Union[Declare, DeprecatedDeclare]:
    """Convert RpcBroadcastedDeclareTxn to Declare or DeprecatedDeclare"""
    if int(declare_transaction["version"], 0) == SUPPORTED_RPC_DECLARE_TX_VERSION:
        return make_declare_v2(declare_transaction)
    if int(declare_transaction["version"], 0) == DEPRECATED_RPC_DECLARE_TX_VERSION:
        return make_declare_v1(declare_transaction)

    raise RpcError(
        code=-1,
        message=f"Declare transaction version {declare_transaction['version']} is not supported by devnet",
    )


def make_deploy_account(
    deploy_account_transaction: RpcBroadcastedDeployAccountTxn,
) -> DeployAccount:
    """
    Convert RpcBroadcastedDeployAccountTxn to DeployAccount
    """
    deploy_account_tx = DeployAccount(
        class_hash=int(deploy_account_transaction["class_hash"], 16),
        contract_address_salt=int(
            deploy_account_transaction["contract_address_salt"], 16
        ),
        constructor_calldata=[
            int(data, 16) for data in deploy_account_transaction["constructor_calldata"]
        ],
        version=int(deploy_account_transaction["version"], 16),
        nonce=int(deploy_account_transaction["nonce"], 16),
        max_fee=int(deploy_account_transaction["max_fee"], 16),
        signature=[int(sig, 16) for sig in deploy_account_transaction["signature"]],
    )
    return deploy_account_tx


class DeprecatedEntryPoint(TypedDict):
    """TypedDict for deprecated rpc contract class entry point"""

    offset: NumAsHex
    selector: Felt


class DeprecatedEntryPoints(TypedDict):
    """TypedDict for deprecated rpc contract class entry points"""

    CONSTRUCTOR: List[DeprecatedEntryPoint]
    EXTERNAL: List[DeprecatedEntryPoint]
    L1_HANDLER: List[DeprecatedEntryPoint]


class SierraEntryPoint(TypedDict):
    """TypedDict for sierra entry point"""

    selector: Felt
    function_idx: int


class EntryPoints(TypedDict):
    """TypedDict for rpc contract class entry points"""

    CONSTRUCTOR: List[SierraEntryPoint]
    EXTERNAL: List[SierraEntryPoint]
    L1_HANDLER: List[SierraEntryPoint]


FunctionAbiType = Literal["function", "l1_handler"]
EventAbiType = Literal["event"]
StructAbiType = Literal["struct"]


class TypedParameter(TypedDict):
    """TypedDict for TypedParameter"""

    name: str
    type: str


class StructMember(TypedDict):
    """TypedDict for StructMember"""

    name: str
    type: str
    offset: int


class FunctionAbiEntry(TypedDict):
    """TypedDict for FunctionAbiEntry"""

    type: FunctionAbiType
    name: str
    inputs: List[TypedParameter]
    outputs: List[TypedParameter]


class EventAbiEntry(TypedDict):
    """TypedDict for EventAbiEntry"""

    type: EventAbiType
    name: str
    keys: List[TypedParameter]
    data: List[TypedParameter]


class StructAbiEntry(TypedDict):
    """TypedDict for StructAbiEntry"""

    type: StructAbiType
    name: str
    size: int  # minimum 1
    members: List[StructMember]


AbiEntry = Union[FunctionAbiEntry, EventAbiEntry, StructAbiEntry]


def function_abi_entry(abi_entry: AbiEntryType) -> FunctionAbiEntry:
    """
    Convert function gateway abi entry to rpc FunctionAbiEntry
    """
    return FunctionAbiEntry(
        type=abi_entry["type"],
        name=abi_entry["name"],
        inputs=abi_entry["inputs"],
        outputs=abi_entry["outputs"],
    )


def struct_abi_entry(abi_entry: AbiEntryType) -> StructAbiEntry:
    """
    Convert struct gateway abi entry to rpc StructAbiEntry
    """
    return StructAbiEntry(
        type=abi_entry["type"],
        name=abi_entry["name"],
        size=abi_entry["size"],
        members=abi_entry["members"],
    )


def event_abi_entry(abi_entry: AbiEntryType) -> EventAbiEntry:
    """
    Convert event gateway abi entry to rpc EventAbiEntry
    """
    return EventAbiEntry(
        type=abi_entry["type"],
        name=abi_entry["name"],
        keys=abi_entry["keys"],
        data=abi_entry["data"],
    )


def rpc_abi_entry(abi_entry: AbiEntryType) -> AbiEntry:
    """
    Convert gateway abi entry to rpc AbiEntry
    """

    function_map = {
        "l1_handler": function_abi_entry,
        "function": function_abi_entry,
        "struct": struct_abi_entry,
        "event": event_abi_entry,
        "constructor": function_abi_entry,
    }

    return function_map[abi_entry["type"]](abi_entry)


class RpcDeprecatedContractClass(TypedDict):
    """
    TypedDict for deprecated rpc contract class
    """

    program: str
    entry_points_by_type: DeprecatedEntryPoints
    abi: Optional[List[AbiEntry]]


class RpcContractClass(TypedDict):
    """
    TypedDict for rpc contract class
    """

    sierra_program: List[Felt]
    contract_class_version: str
    entry_points_by_type: EntryPoints
    abi: Optional[str]


def rpc_deprecated_contract_class(
    contract_class: DeprecatedCompiledClass,
) -> RpcDeprecatedContractClass:
    """
    Convert gateway contract class to rpc contract class
    """

    def program() -> str:
        _program = contract_class.program.Schema().dump(contract_class.program)
        return compress_program(_program)

    def entry_points_by_type() -> DeprecatedEntryPoints:
        _entry_points: DeprecatedEntryPoints = {
            "CONSTRUCTOR": [],
            "EXTERNAL": [],
            "L1_HANDLER": [],
        }
        for typ, entry_points in contract_class.entry_points_by_type.items():
            for entry_point in entry_points:
                _entry_point: DeprecatedEntryPoint = {
                    "selector": rpc_felt(entry_point.selector),
                    "offset": hex(entry_point.offset),
                }
                _entry_points[typ.name].append(_entry_point)
        return _entry_points

    def abi() -> Optional[List[AbiEntry]]:
        if contract_class.abi is None:
            return None
        return [rpc_abi_entry(abi_entry_type) for abi_entry_type in contract_class.abi]

    _contract_class: RpcDeprecatedContractClass = {
        "program": program(),
        "entry_points_by_type": entry_points_by_type(),
        "abi": abi(),
    }
    return _contract_class


def rpc_contract_class(contract_class: ContractClass) -> RpcContractClass:
    """
    Convert gateway contract class v1 to rpc contract class v1
    """

    def program() -> List[Felt]:
        return list(map(rpc_felt, contract_class.sierra_program))

    def entry_points_by_type() -> EntryPoints:
        def map_entry_point(entry_point: ContractEntryPoint) -> SierraEntryPoint:
            return SierraEntryPoint(
                selector=rpc_felt(entry_point.selector),
                function_idx=entry_point.function_idx,
            )

        def get_entry_points_of_type(
            entry_point_type: EntryPointType,
        ) -> List[SierraEntryPoint]:
            return list(
                map(
                    map_entry_point,
                    contract_class.entry_points_by_type[entry_point_type],
                )
            )

        _entry_points: EntryPoints = {
            "CONSTRUCTOR": get_entry_points_of_type(EntryPointType.CONSTRUCTOR),
            "EXTERNAL": get_entry_points_of_type(EntryPointType.EXTERNAL),
            "L1_HANDLER": get_entry_points_of_type(EntryPointType.L1_HANDLER),
        }

        return _entry_points

    _contract_class: RpcContractClass = {
        "sierra_program": program(),
        "entry_points_by_type": entry_points_by_type(),
        "abi": contract_class.abi,
        "contract_class_version": contract_class.contract_class_version,
    }

    return _contract_class


def contract_class_from_dict(
    contract_class_dict: Dict,
) -> Union[RpcContractClass, RpcDeprecatedContractClass]:
    """Convert contract class dict to RpcContractClass or RpcDeprecatedContractClass"""
    if "sierra_program" in contract_class_dict.keys():
        loaded_class = ContractClass.load(contract_class_dict)
        return rpc_contract_class(loaded_class)

    loaded_class = DeprecatedCompiledClass.load(contract_class_dict)
    return rpc_deprecated_contract_class(loaded_class)


class RpcStorageEntry(TypedDict):
    """TypedDict for rpc storage entry"""

    key: Felt
    value: Felt


class RpcStorageDiff(TypedDict):
    """TypedDict for rpc storage diff"""

    address: Felt
    storage_entries: List[RpcStorageEntry]


class RpcDeployedContractDiff(TypedDict):
    """TypedDict for rpc deployed contract diff"""

    address: Felt
    class_hash: Felt


class RpcNonceDiff(TypedDict):
    """TypedDict for rpc nonce diff"""

    contract_address: Address
    nonce: Felt


class RpcDeclaredClass(TypedDict):
    """TypedDict for rpc declared class"""

    class_hash: Felt
    compiled_class_hash: Felt


class RpcReplacedClass(TypedDict):
    """TypedDict for contract which class was replaced"""

    contract_address: Felt
    class_hash: Felt


class RpcStateDiff(TypedDict):
    """TypedDict for rpc state diff"""

    storage_diffs: List[RpcStorageDiff]
    deprecated_declared_classes: List[Felt]
    declared_classes: List[RpcDeclaredClass]
    deployed_contracts: List[RpcDeployedContractDiff]
    replaced_classes: List[RpcReplacedClass]
    nonces: List[RpcNonceDiff]


class RpcPendingStateUpdate(TypedDict):
    """TypedDict for pending rpc state update"""

    old_root: Felt
    state_diff: RpcStateDiff


class RpcStateUpdate(RpcPendingStateUpdate):
    """TypedDict for rpc state update"""

    block_hash: BlockHash
    new_root: Felt


def rpc_state_update(
    state_update: BlockStateUpdate,
) -> Union[RpcStateUpdate, RpcPendingStateUpdate]:
    """
    Convert gateway state update to rpc state update
    """

    def storage_diffs() -> List[RpcStorageDiff]:
        _storage_diffs = []
        for address, diffs in state_update.state_diff.storage_diffs.items():
            storage_entries = []
            for diff in diffs:
                storage_entry = RpcStorageEntry(
                    key=rpc_felt(diff.key), value=rpc_felt(diff.value)
                )
                storage_entries.append(storage_entry)

            _diff = RpcStorageDiff(
                address=rpc_felt(address), storage_entries=storage_entries
            )
            _storage_diffs.append(_diff)
        return _storage_diffs

    def deprecated_declared_classes() -> List[Felt]:
        return [
            rpc_felt(contract)
            for contract in state_update.state_diff.old_declared_contracts
        ]

    def declared_classes() -> List[RpcDeclaredClass]:
        return [
            RpcDeclaredClass(
                class_hash=rpc_felt(class_hash_pair.class_hash),
                compiled_class_hash=rpc_felt(class_hash_pair.compiled_class_hash),
            )
            for class_hash_pair in state_update.state_diff.declared_classes
        ]

    def deployed_contracts() -> List[RpcDeployedContractDiff]:
        _contracts = []
        for contract in state_update.state_diff.deployed_contracts:
            diff: RpcDeployedContractDiff = {
                "address": rpc_felt(contract.address),
                "class_hash": rpc_felt(contract.class_hash),
            }
            _contracts.append(diff)
        return _contracts

    def replaced_classes() -> List[RpcReplacedClass]:
        return [
            RpcReplacedClass(
                contract_address=rpc_felt(replaced_class.address),
                class_hash=rpc_felt(replaced_class.class_hash),
            )
            for replaced_class in state_update.state_diff.replaced_classes
        ]

    def nonces() -> List[RpcNonceDiff]:
        return [
            RpcNonceDiff(contract_address=rpc_felt(address), nonce=rpc_felt(nonce))
            for address, nonce in state_update.state_diff.nonces.items()
        ]

    state_diff: RpcStateDiff = {
        "storage_diffs": storage_diffs(),
        "deprecated_declared_classes": deprecated_declared_classes(),
        "declared_classes": declared_classes(),
        "deployed_contracts": deployed_contracts(),
        "replaced_classes": replaced_classes(),
        "nonces": nonces(),
    }

    if state_update.block_hash is None or state_update.block_hash == 0:
        pending_rpc_state: RpcPendingStateUpdate = {
            "old_root": rpc_root(state_update.old_root.hex()),
            "state_diff": state_diff,
        }
        return pending_rpc_state

    rpc_state: RpcStateUpdate = {
        "block_hash": rpc_felt(state_update.block_hash),
        "new_root": rpc_root(state_update.new_root.hex()),
        "old_root": rpc_root(state_update.old_root.hex()),
        "state_diff": state_diff,
    }
    return rpc_state
