"""
RPC transaction endpoints
"""

import dataclasses
from typing import List

from marshmallow.exceptions import MarshmallowError
from starkware.starknet.services.api.contract_class import ContractClass
from starkware.starknet.services.api.feeder_gateway.response_objects import (
    TransactionStatus,
)
from starkware.starknet.services.api.gateway.transaction import (
    InvokeFunction,
    Declare,
    DEFAULT_DECLARE_SENDER_ADDRESS,
    Deploy,
)
from starkware.starknet.services.api.gateway.transaction_utils import decompress_program
from starkware.starkware_utils.error_handling import StarkException

from starknet_devnet.blueprints.rpc.utils import (
    get_block_by_block_id,
    rpc_felt,
    assert_block_id_is_latest,
)
from starknet_devnet.blueprints.rpc.structures.payloads import (
    rpc_transaction,
    RpcTransaction,
    FunctionCall,
    RpcContractClass,
    RpcInvokeTransaction,
    make_invoke_function,
    rpc_fee_estimate,
)
from starknet_devnet.blueprints.rpc.structures.responses import (
    rpc_transaction_receipt,
    RpcInvokeTransactionResult,
    RpcDeclareTransactionResult,
    RpcDeployTransactionResult,
)
from starknet_devnet.blueprints.rpc.structures.types import (
    TxnHash,
    BlockId,
    NumAsHex,
    Felt,
    RpcError,
)
from starknet_devnet.constants import SUPPORTED_RPC_TX_VERSION
from starknet_devnet.state import state
from starknet_devnet.util import StarknetDevnetException


async def get_transaction_by_hash(transaction_hash: TxnHash) -> dict:
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


async def get_transaction_by_block_id_and_index(block_id: BlockId, index: int) -> dict:
    """
    Get the details of a transaction by a given block id and index
    """
    block = get_block_by_block_id(block_id)

    try:
        transaction_hash: int = block.transactions[index].transaction_hash
    except IndexError as ex:
        raise RpcError(code=27, message="Invalid transaction index in a block") from ex

    return await get_transaction_by_hash(transaction_hash=rpc_felt(transaction_hash))


async def get_transaction_receipt(transaction_hash: TxnHash) -> dict:
    """
    Get the transaction receipt by the transaction hash
    """
    try:
        result = state.starknet_wrapper.transactions.get_transaction_receipt(
            tx_hash=transaction_hash
        )
    except StarknetDevnetException as ex:
        raise RpcError(code=25, message="Invalid transaction hash") from ex

    if result.status == TransactionStatus.NOT_RECEIVED:
        raise RpcError(code=25, message="Invalid transaction hash")

    return rpc_transaction_receipt(result)


async def pending_transactions() -> List[RpcTransaction]:
    """
    Returns the transactions in the transaction pool, recognized by this sequencer
    """
    raise NotImplementedError()


async def add_invoke_transaction(
    function_invocation: FunctionCall,
    signature: List[Felt],
    max_fee: NumAsHex,
    version: NumAsHex,
    nonce: NumAsHex = None,
) -> dict:
    """
    Submit a new transaction to be added to the chain
    """
    invoke_function = InvokeFunction(
        contract_address=int(function_invocation["contract_address"], 16),
        entry_point_selector=int(function_invocation["entry_point_selector"], 16),
        calldata=[int(data, 16) for data in function_invocation["calldata"]],
        max_fee=int(max_fee, 16),
        version=int(version, 16),
        signature=[int(data, 16) for data in signature]
        if signature is not None
        else [],
        nonce=nonce,
    )

    _, transaction_hash = await state.starknet_wrapper.invoke(
        invoke_function=invoke_function
    )
    return RpcInvokeTransactionResult(
        transaction_hash=rpc_felt(transaction_hash),
    )


async def add_declare_transaction(
    contract_class: RpcContractClass, version: NumAsHex
) -> dict:
    """
    Submit a new class declaration transaction
    """
    try:
        decompressed_program = decompress_program(
            {"contract_class": contract_class}, False
        )
        decompressed_program = decompressed_program["contract_class"]

        contract_definition = ContractClass.load(decompressed_program)
        # Replace None with [] in abi key to avoid Missing Abi exception
        contract_definition = dataclasses.replace(contract_definition, abi=[])
    except (StarkException, TypeError, MarshmallowError) as ex:
        raise RpcError(code=50, message="Invalid contract class") from ex

    declare_transaction = Declare(
        contract_class=contract_definition,
        version=int(version, 16),
        sender_address=DEFAULT_DECLARE_SENDER_ADDRESS,
        max_fee=0,
        signature=[],
        nonce=0,
    )

    class_hash, transaction_hash = await state.starknet_wrapper.declare(
        declare_transaction=declare_transaction
    )
    return RpcDeclareTransactionResult(
        transaction_hash=rpc_felt(transaction_hash),
        class_hash=rpc_felt(class_hash),
    )


async def add_deploy_transaction(
    contract_address_salt: Felt,
    constructor_calldata: List[Felt],
    contract_definition: RpcContractClass,
) -> dict:
    """
    Submit a new deploy contract transaction
    """
    try:
        decompressed_program = decompress_program(
            {"contract_definition": contract_definition}, False
        )
        decompressed_program = decompressed_program["contract_definition"]

        contract_class = ContractClass.load(decompressed_program)
        contract_class = dataclasses.replace(contract_class, abi=[])
    except (StarkException, TypeError, MarshmallowError) as ex:
        raise RpcError(code=50, message="Invalid contract class") from ex

    deploy_transaction = Deploy(
        contract_address_salt=int(contract_address_salt, 16),
        constructor_calldata=[int(data, 16) for data in constructor_calldata],
        contract_definition=contract_class,
        version=SUPPORTED_RPC_TX_VERSION,
    )

    contract_address, transaction_hash = await state.starknet_wrapper.deploy(
        deploy_transaction=deploy_transaction
    )
    return RpcDeployTransactionResult(
        transaction_hash=rpc_felt(transaction_hash),
        contract_address=rpc_felt(contract_address),
    )


async def estimate_fee(request: RpcInvokeTransaction, block_id: BlockId) -> dict:
    """
    Estimate the fee for a given StarkNet transaction
    """
    assert_block_id_is_latest(block_id)

    if not state.starknet_wrapper.contracts.is_deployed(
        int(request["contract_address"], 16)
    ):
        raise RpcError(code=20, message="Contract not found")

    invoke_function = make_invoke_function(request)

    try:
        _, fee_response = await state.starknet_wrapper.calculate_trace_and_fee(
            invoke_function
        )
    except StarkException as ex:
        if (
            f"Entry point {hex(int(request['entry_point_selector'], 16))} not found"
            in ex.message
        ):
            raise RpcError(code=21, message="Invalid message selector") from ex
        if "While handling calldata" in ex.message:
            raise RpcError(code=22, message="Invalid call data") from ex
        raise RpcError(code=-1, message=ex.message) from ex
    return rpc_fee_estimate(fee_response)
