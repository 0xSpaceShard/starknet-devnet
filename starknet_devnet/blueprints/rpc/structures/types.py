"""
RPC types
"""

import json
from enum import Enum
from typing import List, Union

from starkware.starknet.definitions.error_codes import StarknetErrorCode
from starkware.starknet.definitions.transaction_type import TransactionType
from starkware.starknet.services.api.feeder_gateway.response_objects import BlockStatus
from starkware.starkware_utils.error_handling import StarkException
from typing_extensions import Literal, TypedDict

from ..rpc_spec import RPC_SPECIFICATION
from ..rpc_spec_write import RPC_SPECIFICATION_WRITE

Felt = str

BlockHash = Felt
BlockNumber = int
BlockTag = Literal["latest", "pending"]

Signature = List[Felt]


class BlockHashDict(TypedDict):
    """TypedDict class for BlockId with block hash"""

    block_hash: BlockHash


class BlockNumberDict(TypedDict):
    """TypedDict class for BlockId with block number"""

    block_number: BlockNumber


BlockId = Union[BlockHashDict, BlockNumberDict, BlockTag]

TxnStatus = Literal["PENDING", "ACCEPTED_ON_L2", "ACCEPTED_ON_L1", "REJECTED"]

RpcBlockStatus = Literal["PENDING", "ACCEPTED_ON_L2", "ACCEPTED_ON_L1", "REJECTED"]


def rpc_block_status(block_status: str) -> RpcBlockStatus:
    """
    Convert gateway BlockStatus to RpcBlockStatus
    """
    block_status_map = {
        BlockStatus.PENDING.name: "PENDING",
        BlockStatus.ABORTED.name: "REJECTED",
        BlockStatus.REVERTED.name: "REJECTED",
        BlockStatus.ACCEPTED_ON_L2.name: "ACCEPTED_ON_L2",
        BlockStatus.ACCEPTED_ON_L1.name: "ACCEPTED_ON_L1",
    }
    return block_status_map[block_status]


TxnHash = Felt
Address = Felt
NumAsHex = str

RpcTxnType = Literal["DECLARE", "DEPLOY", "INVOKE", "L1_HANDLER", "DEPLOY_ACCOUNT"]


def rpc_txn_type(transaction_type: str) -> RpcTxnType:
    """
    Convert gateway TransactionType name to RPCTxnType
    """
    txn_type_map = {
        TransactionType.DEPLOY.name: "DEPLOY",
        TransactionType.DECLARE.name: "DECLARE",
        TransactionType.INVOKE_FUNCTION.name: "INVOKE",
        TransactionType.L1_HANDLER.name: "L1_HANDLER",
        TransactionType.DEPLOY_ACCOUNT.name: "DEPLOY_ACCOUNT",
    }
    if transaction_type not in txn_type_map:
        raise RpcError(
            code=-1,
            message=f"Current implementation does not support {transaction_type} transaction type",
        )
    return txn_type_map[transaction_type]


class RpcError(Exception):
    """
    Error message returned by rpc
    """

    def __init__(self, code, message):
        super().__init__(message)
        self.code = code
        self.message = message

    @staticmethod
    def from_spec_name(name: str):
        """Create an instance of this class, given only its name"""
        error_dict = RPC_ERRORS[name]
        return RpcError(**error_dict)


class PredefinedRpcErrorCode(Enum):
    """
    Constants used in JSON-RPC protocol
    https://www.jsonrpc.org/specification
    """

    INVALID_REQUEST = -32600
    METHOD_NOT_FOUND = -32601
    INVALID_PARAMS = -32602
    INTERNAL_ERROR = -32603


def _combine_rpc_errors():
    """
    Merge write api errors with main api errors.

    All references from write api will be shadowed by errors from main api.
    """
    rpc_errors = json.loads(RPC_SPECIFICATION)["components"]["errors"]
    rpc_write_errors = json.loads(RPC_SPECIFICATION_WRITE)["components"]["errors"]

    return rpc_write_errors | rpc_errors


RPC_ERRORS = _combine_rpc_errors()


def map_gateway_to_rpc_error_dict(exception: StarkException) -> RpcError:
    """
    JSON-RPC cannot work with raw StarkExceptions, they have to be properly mapped
    Contains errors from rpc spec 0.4.0 - necessary for proper mapping of errors
    from Starknet 0.12.1 (mostly validation related). Those custom definitions be
    removed if RPC 0.4.0 support is added - they be supported via the spec file.
    """
    return {
        StarknetErrorCode.BLOCK_NOT_FOUND: RPC_ERRORS["BLOCK_NOT_FOUND"],
        StarknetErrorCode.INSUFFICIENT_MAX_FEE: {
            "code": 53,
            "message": "Max fee is smaller than the minimal transaction cost (validation plus fee transfer)",
        },
        StarknetErrorCode.INSUFFICIENT_ACCOUNT_BALANCE: {
            "code": 54,
            "message": "Account balance is smaller than the transaction's max_fee",
        },
        StarknetErrorCode.VALIDATE_FAILURE: {
            "code": 55,
            "message": "Account validation failed",
        },
        StarknetErrorCode.INVALID_TRANSACTION_NONCE: {
            "code": 52,
            "message": "Invalid transaction nonce",
        },
        StarknetErrorCode.UNDECLARED_CLASS: RPC_ERRORS["CLASS_HASH_NOT_FOUND"],
    }.get(exception.code) or {
        "code": PredefinedRpcErrorCode.INTERNAL_ERROR.value,
        "message": f"Internal error occurred: {exception}",
    }
