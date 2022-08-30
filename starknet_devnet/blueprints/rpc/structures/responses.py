"""
RPC response structures
"""

from typing import List, Optional, Union
from typing_extensions import TypedDict

from starkware.starknet.definitions.transaction_type import TransactionType
from starkware.starknet.services.api.feeder_gateway.response_objects import (
    TransactionReceipt,
    TransactionStatus,
)

from starknet_devnet.blueprints.rpc.utils import rpc_felt
from starknet_devnet.blueprints.rpc.structures.types import (
    TxnHash,
    Felt,
    Address,
    BlockNumber,
    BlockHash,
    TxnStatus,
)
from starknet_devnet.state import state


class RpcInvokeTransactionResult(TypedDict):
    """
    TypedDict for rpc invoke transaction result
    """

    transaction_hash: TxnHash


class RpcDeclareTransactionResult(TypedDict):
    """
    TypedDict for rpc declare transaction result
    """

    transaction_hash: TxnHash
    class_hash: Felt


class RpcDeployTransactionResult(TypedDict):
    """
    TypedDict for rpc deploy transaction result
    """

    transaction_hash: TxnHash
    contract_address: Felt


class MessageToL1(TypedDict):
    """
    TypedDict for rpc message from l2 to l1
    """

    to_address: Felt
    payload: List[Felt]


class MessageToL2(TypedDict):
    """
    TypedDict for rpc message from l1 to l2
    """

    from_address: str
    payload: List[Felt]


class Event(TypedDict):
    """
    TypedDict for rpc event
    """

    from_address: Address
    keys: List[Felt]
    data: List[Felt]


class RpcBaseTransactionReceipt(TypedDict):
    """
    TypedDict for rpc transaction receipt
    """

    # Common
    transaction_hash: TxnHash
    actual_fee: Felt
    status: TxnStatus
    status_data: Optional[str]
    block_hash: BlockHash
    block_number: BlockNumber


class RpcInvokeReceipt(TypedDict):
    """
    TypedDict for rpc invoke transaction receipt
    """

    messages_sent: List[MessageToL1]
    l1_origin_message: Optional[MessageToL2]
    events: List[Event]
    # Common
    transaction_hash: TxnHash
    actual_fee: Felt
    status: TxnStatus
    status_data: Optional[str]
    block_hash: BlockHash
    block_number: BlockNumber


class RpcDeclareReceipt(TypedDict):
    """
    TypedDict for rpc declare transaction receipt
    """

    # Common
    transaction_hash: TxnHash
    actual_fee: Felt
    status: TxnStatus
    status_data: Optional[str]
    block_hash: BlockHash
    block_number: BlockNumber


class RpcDeployReceipt(TypedDict):
    """
    TypedDict for rpc declare transaction receipt
    """

    # Common
    transaction_hash: TxnHash
    actual_fee: Felt
    status: TxnStatus
    status_data: Optional[str]
    block_hash: BlockHash
    block_number: BlockNumber


def rpc_invoke_receipt(txr: TransactionReceipt) -> RpcInvokeReceipt:
    """
    Convert rpc invoke transaction receipt to rpc format
    """

    def l2_to_l1_messages() -> List[MessageToL1]:
        return [
            {
                "to_address": rpc_felt(message.to_address),
                "payload": [rpc_felt(p) for p in message.payload],
            }
            for message in txr.l2_to_l1_messages
        ]

    def l1_to_l2_message() -> Optional[MessageToL2]:
        if txr.l1_to_l2_consumed_message is None:
            return None

        msg: MessageToL2 = {
            "from_address": txr.l1_to_l2_consumed_message.from_address,
            "payload": [rpc_felt(p) for p in txr.l1_to_l2_consumed_message.payload],
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
        **base_receipt,
    }
    return receipt


def rpc_declare_receipt(txr) -> RpcDeclareReceipt:
    """
    Convert rpc declare transaction receipt to rpc format
    """
    return rpc_base_transaction_receipt(txr)


def rpc_deploy_receipt(txr) -> RpcDeployReceipt:
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
        "transaction_hash": rpc_felt(txr.transaction_hash),
        "actual_fee": rpc_felt(txr.actual_fee or 0),
        "status": status(),
        "status_data": status_data(),
        "block_hash": rpc_felt(txr.block_hash) if txr.block_hash is not None else None,
        "block_number": txr.block_number,
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
    transaction = state.starknet_wrapper.transactions.get_transaction(
        hex(txr.transaction_hash)
    ).transaction
    tx_type = transaction.tx_type
    return tx_mapping[tx_type](txr)
