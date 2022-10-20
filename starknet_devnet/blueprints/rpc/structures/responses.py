"""
RPC response structures
"""

from typing import List, TypedDict

from starkware.starknet.definitions.transaction_type import TransactionType
from starkware.starknet.services.api.feeder_gateway.response_objects import (
    TransactionReceipt,
    TransactionStatus,
)

from starknet_devnet.blueprints.rpc.structures.types import (
    TxnHash,
    Felt,
    Address,
    BlockNumber,
    BlockHash,
    TxnStatus,
    TxnType,
    rpc_txn_type,
)
from starknet_devnet.blueprints.rpc.utils import rpc_felt
from starknet_devnet.state import state


class RpcInvokeTransactionResult(TypedDict):
    """TypedDict for rpc invoke transaction result"""

    transaction_hash: TxnHash


class RpcDeclareTransactionResult(TypedDict):
    """TypedDict for rpc declare transaction result"""

    transaction_hash: TxnHash
    class_hash: Felt


class RpcDeployTransactionResult(TypedDict):
    """TypedDict for rpc deploy transaction result"""

    transaction_hash: TxnHash
    contract_address: Felt


class MessageToL1(TypedDict):
    """TypedDict for rpc message from l2 to l1"""

    to_address: Felt
    payload: List[Felt]


class Event(TypedDict):
    """TypedDict for rpc event"""

    from_address: Address
    keys: List[Felt]
    data: List[Felt]


class RpcEventsResult(TypedDict):
    """
    TypedDict for rpc get events result
    """

    events: List[Event]
    continuation_token: str


class RpcBaseTransactionReceipt(TypedDict):
    """TypedDict for rpc transaction receipt"""

    transaction_hash: TxnHash
    actual_fee: Felt
    status: TxnStatus
    block_hash: BlockHash
    block_number: BlockNumber
    type: TxnType
    messages_sent: List[MessageToL1]
    events: List[Event]


RpcInvokeReceipt = RpcBaseTransactionReceipt
RpcDeclareReceipt = RpcBaseTransactionReceipt
RpcL1HandlerReceipt = RpcBaseTransactionReceipt


class RpcDeployReceipt(RpcBaseTransactionReceipt):
    """TypedDict for rpc declare transaction receipt"""

    contract_address: Felt


def rpc_invoke_receipt(txr: TransactionReceipt) -> RpcInvokeReceipt:
    """
    Convert rpc invoke transaction receipt to rpc format
    """
    return rpc_base_transaction_receipt(txr)


def rpc_declare_receipt(txr: TransactionReceipt) -> RpcDeclareReceipt:
    """
    Convert rpc declare transaction receipt to rpc format
    """
    return rpc_base_transaction_receipt(txr)


def rpc_deploy_receipt(txr: TransactionReceipt) -> RpcDeployReceipt:
    """
    Convert rpc deploy transaction receipt to rpc format
    """
    base_receipt = rpc_base_transaction_receipt(txr)
    transaction = state.starknet_wrapper.transactions.get_transaction(
        hex(txr.transaction_hash)
    ).transaction

    receipt: RpcDeployReceipt = {
        "contract_address": rpc_felt(transaction.contract_address),
        **base_receipt,
    }
    return receipt


def rpc_l1_handler_receipt(txr: TransactionReceipt) -> RpcL1HandlerReceipt:
    """
    Convert rpc l1 handler transaction receipt to rpc format
    """
    return rpc_base_transaction_receipt(txr)


def rpc_base_transaction_receipt(txr: TransactionReceipt) -> dict:
    """
    Convert gateway transaction receipt to rpc base transaction receipt
    """

    def messages_sent() -> List[MessageToL1]:
        return [
            {
                "to_address": rpc_felt(message.to_address),
                "payload": [rpc_felt(p) for p in message.payload],
            }
            for message in txr.l2_to_l1_messages
        ]

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

    def txn_type() -> TxnType:
        transaction = state.starknet_wrapper.transactions.get_transaction(
            hex(txr.transaction_hash)
        ).transaction
        return rpc_txn_type(transaction.tx_type.name)

    receipt: RpcBaseTransactionReceipt = {
        "transaction_hash": rpc_felt(txr.transaction_hash),
        "actual_fee": rpc_felt(txr.actual_fee or 0),
        "status": status(),
        "block_hash": rpc_felt(txr.block_hash) if txr.block_hash is not None else None,
        "block_number": txr.block_number or None,
        "messages_sent": messages_sent(),
        "events": events(),
        "type": txn_type(),
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
        TransactionType.L1_HANDLER: rpc_l1_handler_receipt,
    }
    transaction = state.starknet_wrapper.transactions.get_transaction(
        hex(txr.transaction_hash)
    ).transaction
    tx_type = transaction.tx_type
    return tx_mapping[tx_type](txr)
