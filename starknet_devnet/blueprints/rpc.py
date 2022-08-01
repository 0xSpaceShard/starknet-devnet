"""
RPC routes
rpc version: 0.15.0
"""
# pylint: disable=too-many-lines

from __future__ import annotations
import dataclasses
import json

from typing import Callable, Union, List, Tuple, Optional, Any
from typing_extensions import TypedDict
from flask import Blueprint, request
from marshmallow.exceptions import MarshmallowError

from starkware.starknet.services.api.contract_class import ContractClass
from starkware.starkware_utils.error_handling import StarkException
from starkware.starknet.definitions import constants
from starkware.starknet.services.api.gateway.transaction import (
    DECLARE_SENDER_ADDRESS,
    Declare,
    Deploy,
    InvokeFunction,
)
from starkware.starknet.services.api.gateway.transaction_utils import compress_program, decompress_program
from starkware.starknet.services.api.feeder_gateway.response_objects import (
    StarknetBlock,
    InvokeSpecificInfo,
    DeploySpecificInfo,
    TransactionReceipt,
    TransactionStatus,
    TransactionSpecificInfo,
    TransactionType,
    BlockStateUpdate,
    DeclareSpecificInfo
)

from starknet_devnet.state import state
from ..util import StarknetDevnetException

rpc = Blueprint("rpc", __name__, url_prefix="/rpc")

PROTOCOL_VERSION = "0.15.0"


@rpc.route("", methods=["POST"])
async def base_route():
    """
    Base route for RPC calls
    """
    method, args, message_id = parse_body(request.json)

    try:
        result = await method(*args) if isinstance(args, list) else await method(**args)
    except NotImplementedError:
        return rpc_error(message_id=message_id, code=-2, message="Method not implemented")
    except RpcError as error:
        return rpc_error(message_id=message_id, code=error.code, message=error.message)

    return rpc_response(message_id=message_id, content=result)


async def get_block_by_hash(block_hash: str, requested_scope: str = "TXN_HASH") -> dict:
    """
    Get block information given the block id
    """
    try:
        result = state.starknet_wrapper.blocks.get_by_hash(block_hash=block_hash)
    except StarknetDevnetException as ex:
        raise RpcError(code=24, message="Invalid block hash") from ex

    return await rpc_block(block=result, requested_scope=requested_scope)


async def get_block_by_number(block_number: int, requested_scope: str = "TXN_HASH") -> dict:
    """
    Get block information given the block number (its height)
    """
    try:
        result = state.starknet_wrapper.blocks.get_by_number(block_number=block_number)
    except StarknetDevnetException as ex:
        raise RpcError(code=26, message="Invalid block number") from ex

    return await rpc_block(block=result, requested_scope=requested_scope)


async def get_state_update_by_hash(block_hash: str) -> dict:
    """
    Get the information about the result of executing the requested block
    """
    try:
        result = state.starknet_wrapper.blocks.get_state_update(block_hash=block_hash)
    except StarknetDevnetException as ex:
        raise RpcError(code=24, message="Invalid block hash") from ex

    return rpc_state_update(result)


async def get_storage_at(contract_address: str, key: str, block_hash: str) -> str:
    """
    Get the value of the storage at the given address and key
    """
    if block_hash != "latest":
        # By RPC here we should return `24 invalid block hash` but in this case I believe it's more
        # descriptive to the user to use a custom error
        raise RpcError(code=-1, message="Calls with block_hash != 'latest' are not supported currently.")

    if not state.starknet_wrapper.contracts.is_deployed(int(contract_address, 16)):
        raise RpcError(code=20, message="Contract not found")

    return await state.starknet_wrapper.get_storage_at(
        contract_address=int(contract_address, 16),
        key=int(key, 16)
    )


async def get_transaction_by_hash(transaction_hash: str) -> dict:
    """
    Get the details and status of a submitted transaction
    """
    try:
        result = state.starknet_wrapper.transactions.get_transaction(transaction_hash)
    except StarknetDevnetException as ex:
        raise RpcError(code=25, message="Invalid transaction hash") from ex

    if result.status == TransactionStatus.NOT_RECEIVED:
        raise RpcError(code=25, message="Invalid transaction hash")

    return rpc_transaction(result.transaction)


async def get_transaction_by_block_hash_and_index(block_hash: str, index: int) -> dict:
    """
    Get the details of a transaction by a given block hash and index
    """
    try:
        block = state.starknet_wrapper.blocks.get_by_hash(block_hash=block_hash)
    except StarknetDevnetException as ex:
        raise RpcError(code=24, message="Invalid block hash") from ex

    try:
        transaction_hash: int = block.transactions[index].transaction_hash
        return await get_transaction_by_hash(transaction_hash=rpc_felt(transaction_hash))
    except IndexError as ex:
        raise RpcError(code=27, message="Invalid transaction index in a block") from ex


async def get_transaction_by_block_number_and_index(block_number: int, index: int) -> dict:
    """
    Get the details of a transaction by a given block number and index
    """
    try:
        block = state.starknet_wrapper.blocks.get_by_number(block_number=block_number)
    except StarknetDevnetException as ex:
        raise RpcError(code=26, message="Invalid block number") from ex

    try:
        transaction_hash: int = block.transactions[index].transaction_hash
        return await get_transaction_by_hash(transaction_hash=rpc_felt(transaction_hash))
    except IndexError as ex:
        raise RpcError(code=27, message="Invalid transaction index in a block") from ex


async def get_transaction_receipt(transaction_hash: str) -> dict:
    """
    Get the transaction receipt by the transaction hash
    """
    try:
        result = state.starknet_wrapper.transactions.get_transaction_receipt(tx_hash=transaction_hash)
    except StarknetDevnetException as ex:
        raise RpcError(code=25, message="Invalid transaction hash") from ex

    if result.status == TransactionStatus.NOT_RECEIVED:
        raise RpcError(code=25, message="Invalid transaction hash")

    return rpc_transaction_receipt(result)


async def get_code(contract_address: str) -> dict:
    """
    Get the code of a specific contract
    """
    try:
        result = state.starknet_wrapper.contracts.get_code(address=int(contract_address, 16))
    except StarknetDevnetException as ex:
        raise RpcError(code=20, message="Contract not found") from ex

    if len(result["bytecode"]) == 0:
        raise RpcError(code=20, message="Contract not found")

    return {
        "bytecode": result["bytecode"],
        "abi": json.dumps(result["abi"])
    }


async def get_class(class_hash: str) -> dict:
    """
    Get the code of a specific contract
    """
    try:
        result = state.starknet_wrapper.contracts.get_class_by_hash(class_hash=int(class_hash, 16))
    except StarknetDevnetException as ex:
        raise RpcError(code=28, message="The supplied contract class hash is invalid or unknown") from ex

    return rpc_contract_class(result)


async def get_class_hash_at(contract_address: str) -> str:
    """
    Get the contract class hash for the contract deployed at the given address
    """
    try:
        result = state.starknet_wrapper.contracts.get_class_hash_at(address=int(contract_address, 16))
    except StarknetDevnetException as ex:
        raise RpcError(code=28, message="The supplied contract class hash is invalid or unknown") from ex

    return rpc_felt(result)


async def get_class_at(contract_address: str) -> dict:
    """
    Get the contract class definition at the given address
    """
    try:
        class_hash = state.starknet_wrapper.contracts.get_class_hash_at(address=int(contract_address, 16))
        result = state.starknet_wrapper.contracts.get_class_by_hash(class_hash=class_hash)
    except StarknetDevnetException as ex:
        raise RpcError(code=20, message="Contract not found") from ex

    return rpc_contract_class(result)


async def get_block_transaction_count_by_hash(block_hash: str) -> int:
    """
    Get the number of transactions in a block given a block hash
    """
    try:
        block = state.starknet_wrapper.blocks.get_by_hash(block_hash=block_hash)
        return len(block.transactions)
    except StarknetDevnetException as ex:
        raise RpcError(code=24, message="Invalid block hash") from ex


async def get_block_transaction_count_by_number(block_number: int) -> int:
    """
    Get the number of transactions in a block given a block number (height)
    """
    try:
        block = state.starknet_wrapper.blocks.get_by_number(block_number=block_number)
        return len(block.transactions)
    except StarknetDevnetException as ex:
        raise RpcError(code=26, message="Invalid block number") from ex


async def call(contract_address: str, entry_point_selector: str, calldata: list, block_hash: str = "") -> list:
    """
    Call a starknet function without creating a StarkNet transaction
    """
    request_body = {
        "contract_address": contract_address,
        "entry_point_selector": entry_point_selector,
        "calldata": calldata
    }

    # For now, we only support 'latest' block, support for specific blocks
    # in devnet is more complicated if possible at all
    if block_hash != "latest":
        # By RPC here we should return `24 invalid block hash` but in this case I believe it's more
        # descriptive to the user to use a custom error
        raise RpcError(code=-1, message="Calls with block_hash != 'latest' are not supported currently.")

    if not state.starknet_wrapper.contracts.is_deployed(int(contract_address, 16)):
        raise RpcError(code=20, message="Contract not found")

    try:
        return await state.starknet_wrapper.call(transaction=make_invoke_function(request_body))
    except StarknetDevnetException as ex:
        raise RpcError(code=-1, message=ex.message) from ex
    except StarkException as ex:
        if f"Entry point {entry_point_selector} not found" in ex.message:
            raise RpcError(code=21, message="Invalid message selector") from ex
        if "While handling calldata" in ex.message:
            raise RpcError(code=22, message="Invalid call data") from ex
        raise RpcError(code=-1, message=ex.message) from ex


async def estimate_fee():
    """
    Get the estimate fee for the transaction
    """
    raise NotImplementedError()


async def get_block_number() -> int:
    """
    Get the most recent accepted block number
    """
    result = state.starknet_wrapper.blocks.get_number_of_blocks() - 1
    return result if result >= 0 else 0


async def chain_id() -> str:
    """
    Return the currently configured StarkNet chain id
    """
    devnet_state = await state.starknet_wrapper.get_state()
    config = devnet_state.general_config
    chain: int = config.chain_id.value
    return hex(chain)


async def pending_transactions():
    """
    Returns the transactions in the transaction pool, recognized by this sequencer
    """
    raise NotImplementedError()


async def protocol_version() -> str:
    """
    Returns the current starknet protocol version identifier, as supported by this sequencer
    """
    protocol_hex = PROTOCOL_VERSION.encode("utf-8").hex()
    return "0x" + protocol_hex


async def syncing():
    """
    Returns an object about the sync status, or false if the node is not synching
    """
    raise NotImplementedError()


async def get_events():
    """
    Returns all events matching the given filter
    """
    raise NotImplementedError()


async def add_invoke_transaction(function_invocation: dict, signature: List[str], max_fee: str, version: str) -> dict:
    """
    Submit a new transaction to be added to the chain
    """
    invoke_function = InvokeFunction(
        contract_address=int(function_invocation["contract_address"], 16),
        entry_point_selector=int(function_invocation["entry_point_selector"], 16),
        calldata=[int(data, 16) for data in function_invocation["calldata"]],
        max_fee=int(max_fee, 16),
        version=int(version, 16),
        signature=[int(data, 16) for data in signature],
    )

    _, transaction_hash, _ = await state.starknet_wrapper.invoke(invoke_function=invoke_function)
    return RpcInvokeTransactionResult(
        transaction_hash=rpc_felt(transaction_hash),
    )


async def add_declare_transaction(contract_class: RpcContractClass, version: str) -> dict:
    """
    Submit a new class declaration transaction
    """
    try:
        decompressed_program = decompress_program({"contract_class": contract_class}, False)["contract_class"]
        contract_definition = ContractClass.load(decompressed_program)

        # Replace None with [] in abi key to avoid Missing Abi exception
        contract_definition = dataclasses.replace(contract_definition, abi=[])
    except (StarkException, TypeError, MarshmallowError) as ex:
        raise RpcError(code=50, message="Invalid contract class") from ex

    declare_transaction = Declare(
        contract_class=contract_definition,
        version=int(version, 16),
        sender_address=DECLARE_SENDER_ADDRESS,
        max_fee=0,
        signature=[],
        nonce=0,
    )

    class_hash, transaction_hash = await state.starknet_wrapper.declare(declare_transaction=declare_transaction)
    return RpcDeclareTransactionResult(
        transaction_hash=rpc_felt(transaction_hash),
        class_hash=rpc_felt(class_hash),
    )


async def add_deploy_transaction(contract_address_salt: str, constructor_calldata: List[str], contract_definition: RpcContractClass) -> dict:
    """
    Submit a new deploy contract transaction
    """
    try:
        decompressed_program = decompress_program({"contract_definition": contract_definition}, False)["contract_definition"]
        contract_class = ContractClass.load(decompressed_program)
        contract_class = dataclasses.replace(contract_class, abi=[])
    except (StarkException, TypeError, MarshmallowError) as ex:
        raise RpcError(code=50, message="Invalid contract class") from ex

    deploy_transaction = Deploy(
        contract_address_salt=int(contract_address_salt, 16),
        constructor_calldata=[int(data, 16) for data in constructor_calldata],
        contract_definition=contract_class,
        version=constants.TRANSACTION_VERSION,
    )

    contract_address, transaction_hash = await state.starknet_wrapper.deploy(deploy_transaction=deploy_transaction)
    return RpcDeployTransactionResult(
        transaction_hash=rpc_felt(transaction_hash),
        contract_address=rpc_felt(contract_address),
    )


def make_invoke_function(request_body: dict) -> InvokeFunction:
    """
    Convert RPC request to internal InvokeFunction
    """
    return InvokeFunction(
        contract_address=int(request_body["contract_address"], 16),
        entry_point_selector=int(request_body["entry_point_selector"], 16),
        calldata=[int(data, 16) for data in request_body["calldata"]],
        max_fee=0,
        version=0,
        signature=[],
    )


class EntryPoint(TypedDict):
    """
    TypedDict for rpc contract class entry point
    """
    offset: str
    selector: str


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
                    "offset": rpc_felt(entry_point.offset)
                }
                _entry_points[typ.name].append(_entry_point)
        return _entry_points

    _contract_class: RpcContractClass = {
        "program": program(),
        "entry_points_by_type": entry_points_by_type()
    }
    return _contract_class


class RpcBlock(TypedDict):
    """
    TypeDict for rpc block
    """
    block_hash: str
    parent_hash: str
    block_number: int
    status: str
    sequencer_address: str
    new_root: str
    timestamp: int
    transactions: Union[List[str], List[dict]]


async def rpc_block(block: StarknetBlock, requested_scope: Optional[str] = "TXN_HASH") -> RpcBlock:
    """
    Convert gateway block to rpc block
    """
    async def transactions() -> List[Union[RpcInvokeTransaction, RpcDeclareTransaction]]:
        # pylint: disable=no-member
        return [rpc_transaction(tx) for tx in block.transactions]

    async def transaction_hashes() -> List[str]:
        return [tx["txn_hash"] for tx in await transactions()]

    async def full_transactions() -> list[dict[str, Any]]:
        transactions_and_receipts = []
        _transactions = await transactions()
        for transaction in _transactions:
            receipt = await get_transaction_receipt(transaction["txn_hash"])
            combined = {**receipt, **transaction}
            transactions_and_receipts.append(combined)
        return transactions_and_receipts

    def new_root() -> str:
        # pylint: disable=no-member
        return rpc_root(block.state_root.hex())

    mapping: dict[str, Callable] = {
        "TXN_HASH": transaction_hashes,
        "FULL_TXNS": transactions,
        "FULL_TXN_AND_RECEIPTS": full_transactions,
    }
    transactions: list = await mapping[requested_scope]()

    devnet_state = await state.starknet_wrapper.get_state()
    config = devnet_state.general_config

    block: RpcBlock = {
        "block_hash": rpc_felt(block.block_hash),
        "parent_hash": rpc_felt(block.parent_block_hash) or "0x0",
        "block_number": block.block_number if block.block_number is not None else 0,
        "status": block.status.name,
        "sequencer_address": hex(config.sequencer_address),
        "new_root": new_root(),
        "timestamp": block.timestamp,
        "transactions": transactions,
    }
    return block


class RpcStorageDiff(TypedDict):
    """
    TypedDict for rpc storage diff
    """
    address: str
    key: str
    value: str


class RpcContractDiff(TypedDict):
    """
    TypedDict for rpc contract diff
    """
    address: str
    contract_hash: str


class RpcNonceDiff(TypedDict):
    """
    TypedDict for rpc nonce diff
    """
    contract_address: str
    nonce: str


class RpcStateDiff(TypedDict):
    """
    TypedDict for rpc state diff
    """
    storage_diffs: List[RpcStorageDiff]
    contracts: List[RpcContractDiff]
    nonces: List[RpcNonceDiff]


class RpcStateUpdate(TypedDict):
    """
    TypedDict for rpc state update
    """
    block_hash: str
    new_root: str
    old_root: str
    accepted_time: int
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

    def contracts() -> List[RpcContractDiff]:
        _contracts = []
        for contract in state_update.state_diff.deployed_contracts:
            diff: RpcContractDiff = {
                "address": rpc_felt(contract.address),
                "contract_hash": rpc_felt(contract.class_hash)
            }
            _contracts.append(diff)
        return _contracts

    def timestamp() -> int:
        block = state.starknet_wrapper.blocks.get_by_hash(block_hash=hex(state_update.block_hash))
        return block.timestamp

    rpc_state: RpcStateUpdate = {
        "block_hash": rpc_felt(state_update.block_hash),
        "new_root": rpc_root(state_update.new_root.hex()),
        "old_root": rpc_root(state_update.old_root.hex()),
        "accepted_time": timestamp(),
        "state_diff": {
            "storage_diffs": storage_diffs(),
            "contracts": contracts(),
            "nonces": [],
        }
    }
    return rpc_state


def rpc_state_diff_contract(contract: dict) -> dict:
    """
    Convert gateway contract state diff to rpc contract state diff
    """
    return {
        "address": contract["address"],
        "contract_hash": f"0x{contract['contract_hash']}",
    }


def rpc_state_diff_storage(contract: dict) -> dict:
    """
    Convert gateway storage state diff to rpc storage state diff
    """
    return {
        "address": contract["address"],
        "key": contract["key"],
        "value": contract["value"],
    }


class RpcInvokeTransaction(TypedDict):
    """
    TypedDict for rpc invoke transaction
    """
    contract_address: str
    entry_point_selector: Optional[str]
    calldata: Optional[List[str]]
    # Common
    txn_hash: str
    max_fee: str
    version: str
    signature: List[str]


class RpcDeclareTransaction(TypedDict):
    """
    TypedDict for rpc declare transaction
    """
    contract_class: RpcContractClass
    sender_address: str
    # Common
    txn_hash: str
    max_fee: str
    version: str
    signature: List[str]


class RpcInvokeTransactionResult(TypedDict):
    """
    TypedDict for rpc invoke transaction result
    """
    transaction_hash: str


class RpcDeclareTransactionResult(TypedDict):
    """
    TypedDict for rpc declare transaction result
    """
    transaction_hash: str
    class_hash: str


class RpcDeployTransactionResult(TypedDict):
    """
    TypedDict for rpc deploy transaction result
    """
    transaction_hash: str
    contract_address: str


def rpc_invoke_transaction(transaction: InvokeSpecificInfo) -> RpcInvokeTransaction:
    """
    Convert gateway invoke transaction to rpc format
    """
    transaction: RpcInvokeTransaction = {
        "contract_address": rpc_felt(transaction.contract_address),
        "entry_point_selector": rpc_felt(transaction.entry_point_selector),
        "calldata": [rpc_felt(data) for data in transaction.calldata],
        "max_fee": rpc_felt(transaction.max_fee),
        "txn_hash": rpc_felt(transaction.transaction_hash),
        "version": hex(0x0),
        "signature": [rpc_felt(value) for value in transaction.signature]
    }
    return transaction


def rpc_deploy_transaction(transaction: DeploySpecificInfo) -> RpcInvokeTransaction:
    """
    Convert gateway deploy transaction to rpc format
    """
    transaction: RpcInvokeTransaction = {
        "contract_address": rpc_felt(transaction.contract_address),
        "entry_point_selector": None,
        "calldata": [rpc_felt(data) for data in transaction.constructor_calldata],
        "max_fee": rpc_felt(0x0),
        "txn_hash": rpc_felt(transaction.transaction_hash),
        "version": hex(0x0),
        "signature": []
    }
    return transaction


def rpc_declare_transaction(transaction: DeclareSpecificInfo) -> RpcDeclareTransaction:
    """
    Convert gateway declare transaction to rpc format
    """
    def contract_class() -> RpcContractClass:
        # pylint: disable=no-member
        _contract_claass = state.starknet_wrapper.contracts.get_class_by_hash(transaction.class_hash)
        return rpc_contract_class(_contract_claass)

    transaction: RpcDeclareTransaction = {
        "contract_class": contract_class(),
        "sender_address": rpc_felt(transaction.sender_address),
        "max_fee": rpc_felt(transaction.max_fee),
        "txn_hash": rpc_felt(transaction.transaction_hash),
        "version": hex(transaction.version),
        "signature": [rpc_felt(value) for value in transaction.signature]
    }
    return transaction


def rpc_transaction(transaction: TransactionSpecificInfo) -> Union[RpcInvokeTransaction, RpcDeclareTransaction]:
    """
    Convert gateway transaction to rpc transaction
    """
    tx_mapping = {
        TransactionType.DEPLOY: rpc_deploy_transaction,
        TransactionType.INVOKE_FUNCTION: rpc_invoke_transaction,
        TransactionType.DECLARE: rpc_declare_transaction,
    }
    return tx_mapping[transaction.tx_type](transaction)


class MessageToL1(TypedDict):
    """
    TypedDict for rpc message from l2 to l1
    """
    to_address: str
    payload: List[str]


class MessageToL2(TypedDict):
    """
    TypedDict for rpc message from l1 to l2
    """
    from_address: str
    payload: List[str]


class Event(TypedDict):
    """
    TypedDict for rpc event
    """
    from_address: str
    keys: List[str]
    data: List[str]


class RpcBaseTransactionReceipt(TypedDict):
    """
    TypedDict for rpc transaction receipt
    """
    # Common
    txn_hash: str
    actual_fee: str
    status: str
    statusData: Optional[str]


class RpcInvokeReceipt(TypedDict):
    """
    TypedDict for rpc invoke transaction receipt
    """
    messages_sent: List[MessageToL1]
    l1_origin_message: Optional[MessageToL2]
    events: List[Event]
    # Common
    txn_hash: str
    actual_fee: str
    status: str
    statusData: Optional[str]


class RpcDeclareReceipt(TypedDict):
    """
    TypedDict for rpc declare transaction receipt
    """
    # Common
    txn_hash: str
    actual_fee: str
    status: str
    statusData: Optional[str]


def rpc_invoke_receipt(txr: TransactionReceipt) -> RpcInvokeReceipt:
    """
    Convert rpc invoke transaction receipt to rpc format
    """
    def l2_to_l1_messages() -> List[MessageToL1]:
        messages = []
        for message in txr.l2_to_l1_messages:
            msg: MessageToL1 = {
                "to_address": message.to_address,
                "payload": [rpc_felt(p) for p in message.payload]
            }
            messages.append(msg)
        return messages

    def l1_to_l2_message() -> Optional[MessageToL2]:
        if txr.l1_to_l2_consumed_message is None:
            return None

        msg: MessageToL2 = {
            "from_address": txr.l1_to_l2_consumed_message.from_address,
            "payload": [rpc_felt(p) for p in txr.l1_to_l2_consumed_message.payload]
        }
        return msg

    def events() -> List[Event]:
        _events = []
        for event in txr.events:
            event: Event = {
                "from_address": rpc_felt(event.from_address),
                "keys": [rpc_felt(e) for e in event.keys],
                "data": [rpc_felt(d) for d in event.data],
            }
            _events.append(event)
        return _events

    base_receipt = rpc_base_transaction_receipt(txr)
    receipt: RpcInvokeReceipt = {
        "messages_sent": l2_to_l1_messages(),
        "l1_origin_message": l1_to_l2_message(),
        "events": events(),
        "txn_hash": base_receipt["txn_hash"],
        "status": base_receipt["status"],
        "statusData": base_receipt["statusData"],
        "actual_fee": base_receipt["actual_fee"],
    }
    return receipt


def rpc_declare_receipt(txr) -> RpcDeclareReceipt:
    """
    Convert rpc declare transaction receipt to rpc format
    """
    base_receipt = rpc_base_transaction_receipt(txr)
    receipt: RpcDeclareReceipt = {
        "txn_hash": base_receipt["txn_hash"],
        "status": base_receipt["status"],
        "statusData": base_receipt["statusData"],
        "actual_fee": base_receipt["actual_fee"],
    }
    return receipt


def rpc_deploy_receipt(txr) -> RpcBaseTransactionReceipt:
    """
    Convert rpc deploy transaction receipt to rpc format
    """
    return rpc_base_transaction_receipt(txr)


def rpc_base_transaction_receipt(txr: TransactionReceipt) -> RpcBaseTransactionReceipt:
    """
    Convert gateway transaction receipt to rpc transaction receipt
    """
    def status() -> str:
        if txr.status is None:
            return "UNKNOWN"

        mapping = {
            TransactionStatus.NOT_RECEIVED: "UNKNOWN",
            TransactionStatus.ACCEPTED_ON_L2: "ACCEPTED_ON_L2",
            TransactionStatus.ACCEPTED_ON_L1: "ACCEPTED_ON_L1",
            TransactionStatus.RECEIVED: "RECEIVED",
            TransactionStatus.PENDING: "PENDING",
            TransactionStatus.REJECTED: "REJECTED",
        }
        return mapping[txr.status]

    def status_data() -> Union[str, None]:
        if txr.transaction_failure_reason is not None:
            if txr.transaction_failure_reason.error_message is not None:
                return txr.transaction_failure_reason.error_message
        return None

    receipt: RpcBaseTransactionReceipt = {
        "txn_hash": rpc_felt(txr.transaction_hash),
        "status": status(),
        "statusData": status_data(),
        "actual_fee": rpc_felt(txr.actual_fee or 0),
    }
    return receipt


def rpc_transaction_receipt(txr: TransactionReceipt) -> dict:
    """
    Convert gateway transaction receipt to rpc format
    """
    tx_mapping = {
        TransactionType.DEPLOY: rpc_deploy_receipt,
        TransactionType.INVOKE_FUNCTION: rpc_invoke_receipt,
        TransactionType.DECLARE: rpc_declare_receipt,
    }
    transaction = state.starknet_wrapper.transactions.get_transaction(hex(txr.transaction_hash)).transaction
    tx_type = transaction.tx_type
    return tx_mapping[tx_type](txr)


def rpc_response(message_id: int, content: dict) -> dict:
    """
    Wrap response content in rpc format
    """
    return {
        "jsonrpc": "2.0",
        "id": message_id,
        "result": content
    }


def rpc_error(message_id: int, code: int, message: str) -> dict:
    """
    Wrap error in rpc format
    """
    return {
        "jsonrpc": "2.0",
        "id": message_id,
        "error": {
            "code": code,
            "message": message
        }
    }


def rpc_felt(value: int) -> str:
    """
    Convert integer to 0x0 prefixed felt
    """
    return "0x0" + hex(value).lstrip("0x")


def rpc_root(root: str) -> str:
    """
    Convert 0 prefixed root to 0x prefixed root
    """
    root = root[1:]
    return "0x0" + root


def parse_body(body: dict) -> Tuple[Callable, Union[List, dict], int]:
    """
    Parse rpc call body to function name and params
    """
    methods = {
        "getBlockByNumber": get_block_by_number,
        "getBlockByHash": get_block_by_hash,
        "getStateUpdateByHash": get_state_update_by_hash,
        "getStorageAt": get_storage_at,
        "getTransactionByHash": get_transaction_by_hash,
        "getTransactionByBlockHashAndIndex": get_transaction_by_block_hash_and_index,
        "getTransactionByBlockNumberAndIndex": get_transaction_by_block_number_and_index,
        "getTransactionReceipt": get_transaction_receipt,
        "getCode": get_code,
        "getBlockTransactionCountByHash": get_block_transaction_count_by_hash,
        "getBlockTransactionCountByNumber": get_block_transaction_count_by_number,
        "call": call,
        "blockNumber": get_block_number,
        "chainId": chain_id,
        "pendingTransactions": pending_transactions,
        "protocolVersion": protocol_version,
        "syncing": syncing,
        "getEvents": get_events,
        "getClass": get_class,
        "getClassHashAt": get_class_hash_at,
        "getClassAt": get_class_at,
        "estimateFee": estimate_fee,
        "addInvokeTransaction": add_invoke_transaction,
        "addDeclareTransaction": add_declare_transaction,
        "addDeployTransaction": add_deploy_transaction,
    }
    method_name = body["method"].replace("starknet_", "")
    args: Union[List, dict] = body["params"]
    message_id = body["id"]

    if method_name not in methods:
        raise RpcError(code=-1, message="Method not found")

    return methods[method_name], args, message_id


class RpcError(Exception):
    """
    Error message returned by rpc
    """

    def __init__(self, code, message):
        super().__init__(message)
        self.code = code
        self.message = message
