"""
Feeder gateway routes.
"""

from flask import request, jsonify, Blueprint, Response
from marshmallow import ValidationError
from starkware.starknet.services.api.feeder_gateway.response_objects import (
    BlockTransactionTraces,
)
from starkware.starknet.services.api.gateway.transaction import (
    AccountTransaction,
    InvokeFunction,
)
from starkware.starknet.services.api.gateway.transaction import Transaction
from starkware.starknet.services.api.feeder_gateway.request_objects import (
    CallFunction,
    CallL1Handler,
)
from starkware.starknet.services.api.feeder_gateway.response_objects import (
    TransactionSimulationInfo,
)
from werkzeug.datastructures import MultiDict

from starknet_devnet.state import state
from starknet_devnet.util import StarknetDevnetException, custom_int, fixed_length_hex

feeder_gateway = Blueprint("feeder_gateway", __name__, url_prefix="/feeder_gateway")


def validate_request(data: bytes, cls):
    """Ensure `data` is valid Starknet function call. Returns an object of type specified with `cls`."""
    try:
        return cls.loads(data)
    except (TypeError, ValidationError) as err:
        raise StarknetDevnetException(
            message=f"Invalid {cls.__name__}: {err}", status_code=400
        ) from err


def _check_block_hash(request_args: MultiDict):
    block_hash = request_args.get("blockHash", type=custom_int)
    if block_hash is not None:
        print(
            "Specifying a block by its hash is not supported. All interaction is done with the latest block."
        )


def _check_block_arguments(block_hash, block_number):
    if block_hash is not None and block_number is not None:
        message = "Ambiguous criteria: only one of (block number, block hash) can be provided."
        raise StarknetDevnetException(message=message, status_code=500)


def _get_block_object(block_hash: str, block_number: int):
    """Returns the block object"""

    _check_block_arguments(block_hash, block_number)

    if block_hash is not None:
        block = state.starknet_wrapper.blocks.get_by_hash(block_hash)
    else:
        block = state.starknet_wrapper.blocks.get_by_number(block_number)

    return block


def _get_block_transaction_traces(block):
    traces = []
    if block.transaction_receipts:
        for transaction in block.transaction_receipts:
            tx_hash = hex(transaction.transaction_hash)
            trace = state.starknet_wrapper.transactions.get_transaction_trace(tx_hash)

            # expected trace is equal to response of get_transaction, but with the hash property
            trace_dict = trace.dump()
            trace_dict["transaction_hash"] = tx_hash
            traces.append(trace_dict)

    # assert correct structure
    return BlockTransactionTraces.load({"traces": traces})


@feeder_gateway.route("/get_contract_addresses", methods=["GET"])
def get_contract_addresses():
    """Endpoint that returns an object containing the addresses of key system components."""
    return "Not implemented", 501


@feeder_gateway.route("/call_contract", methods=["POST"])
async def call_contract():
    """
    Endpoint for receiving calls (not invokes) of contract functions.
    """

    try:
        call_specifications = validate_request(request.data, CallFunction)  # version 1
    except StarknetDevnetException:
        call_specifications = validate_request(
            request.data, InvokeFunction
        )  # version 0

    result_dict = await state.starknet_wrapper.call(call_specifications)
    return jsonify(result_dict)


@feeder_gateway.route("/get_block", methods=["GET"])
def get_block():
    """Endpoint for retrieving a block identified by its hash or number."""

    block_hash = request.args.get("blockHash")
    block_number = request.args.get("blockNumber", type=custom_int)

    block = _get_block_object(block_hash=block_hash, block_number=block_number)

    return Response(block.dumps(), status=200, mimetype="application/json")


@feeder_gateway.route("/get_block_traces", methods=["GET"])
def get_block_traces():
    """Returns the traces of the transactions in the specified block."""

    block_hash = request.args.get("blockHash")
    block_number = request.args.get("blockNumber", type=custom_int)

    block = _get_block_object(block_hash=block_hash, block_number=block_number)
    block_transaction_traces = _get_block_transaction_traces(block)

    return jsonify(block_transaction_traces.dump())


@feeder_gateway.route("/get_code", methods=["GET"])
def get_code():
    """
    Returns the ABI and bytecode of the contract whose contractAddress is provided.
    """

    _check_block_hash(request.args)

    contract_address = request.args.get("contractAddress", type=custom_int)
    result_dict = state.starknet_wrapper.contracts.get_code(contract_address)
    return jsonify(result_dict)


@feeder_gateway.route("/get_full_contract", methods=["GET"])
def get_full_contract():
    """
    Returns the contract class of the contract whose contractAddress is provided.
    """
    _check_block_hash(request.args)

    contract_address = request.args.get("contractAddress", type=custom_int)

    contract_class = state.starknet_wrapper.contracts.get_full_contract(
        contract_address
    )

    return jsonify(contract_class.dump())


@feeder_gateway.route("/get_class_hash_at", methods=["GET"])
def get_class_hash_at():
    """Get contract class hash by contract address"""

    contract_address = request.args.get("contractAddress", type=custom_int)
    class_hash = state.starknet_wrapper.contracts.get_class_hash_at(contract_address)
    return jsonify(fixed_length_hex(class_hash))


@feeder_gateway.route("/get_class_by_hash", methods=["GET"])
def get_class_by_hash():
    """Get contract class by class hash"""

    class_hash = request.args.get("classHash", type=custom_int)
    contract_class = state.starknet_wrapper.contracts.get_class_by_hash(class_hash)
    return jsonify(contract_class.dump())


@feeder_gateway.route("/get_storage_at", methods=["GET"])
async def get_storage_at():
    """Endpoint for returning the storage identified by `key` from the contract at"""
    _check_block_hash(request.args)

    contract_address = request.args.get("contractAddress", type=custom_int)
    key = request.args.get("key", type=custom_int)

    storage = await state.starknet_wrapper.get_storage_at(contract_address, key)
    return jsonify(storage)


@feeder_gateway.route("/get_transaction_status", methods=["GET"])
def get_transaction_status():
    """
    Returns the status of the transaction identified by the transactionHash argument in the GET request.
    """

    transaction_hash = request.args.get("transactionHash")
    transaction_status = state.starknet_wrapper.transactions.get_transaction_status(
        transaction_hash
    )
    return jsonify(transaction_status)


@feeder_gateway.route("/get_transaction", methods=["GET"])
def get_transaction():
    """
    Returns the transaction identified by the transactionHash argument in the GET request.
    """

    transaction_hash = request.args.get("transactionHash")
    transaction_info = state.starknet_wrapper.transactions.get_transaction(
        transaction_hash
    )
    return Response(
        response=transaction_info.dumps(), status=200, mimetype="application/json"
    )


@feeder_gateway.route("/get_transaction_receipt", methods=["GET"])
def get_transaction_receipt():
    """
    Returns the transaction receipt identified by the transactionHash argument in the GET request.
    """

    transaction_hash = request.args.get("transactionHash")
    transaction_receipt = state.starknet_wrapper.transactions.get_transaction_receipt(
        transaction_hash
    )
    return Response(
        response=transaction_receipt.dumps(), status=200, mimetype="application/json"
    )


@feeder_gateway.route("/get_transaction_trace", methods=["GET"])
def get_transaction_trace():
    """
    Returns the trace of the transaction identified by the transactionHash argument in the GET request.
    """

    transaction_hash = request.args.get("transactionHash")
    transaction_trace = state.starknet_wrapper.transactions.get_transaction_trace(
        transaction_hash
    )

    return Response(
        response=transaction_trace.dumps(), status=200, mimetype="application/json"
    )


@feeder_gateway.route("/get_state_update", methods=["GET"])
def get_state_update():
    """
    Returns the status update from the block identified by the blockHash argument in the GET request.
    If no block hash was provided it will default to the last block.
    """

    block_hash = request.args.get("blockHash")
    block_number = request.args.get("blockNumber", type=custom_int)

    state_update = state.starknet_wrapper.blocks.get_state_update(
        block_hash=block_hash, block_number=block_number
    )

    if state_update is not None:
        return Response(
            response=state_update.dumps(), status=200, mimetype="application/json"
        )

    return jsonify(state_update)


@feeder_gateway.route("/estimate_fee", methods=["POST"])
async def estimate_fee():
    """Returns the estimated fee for a transaction."""

    try:
        transaction = validate_request(request.data, Transaction)  # version 1
    except StarknetDevnetException:
        transaction = validate_request(request.data, InvokeFunction)  # version 0

    _, fee_response = await state.starknet_wrapper.calculate_trace_and_fee(transaction)
    return jsonify(fee_response)


@feeder_gateway.route("/simulate_transaction", methods=["POST"])
async def simulate_transaction():
    """Returns the estimated fee for a transaction."""
    transaction = validate_request(request.data, AccountTransaction)
    trace, fee_response = await state.starknet_wrapper.calculate_trace_and_fee(
        transaction
    )

    simulation_info = TransactionSimulationInfo(
        trace=trace, fee_estimation=fee_response
    )

    return jsonify(simulation_info.dump())


@feeder_gateway.route("/get_nonce", methods=["GET"])
async def get_nonce():
    """Returns the nonce of the contract whose contractAddress is provided"""

    contract_address = request.args.get("contractAddress", type=custom_int)
    nonce = await state.starknet_wrapper.get_nonce(contract_address)

    return jsonify(hex(nonce))


@feeder_gateway.route("/estimate_message_fee", methods=["POST"])
async def estimate_message_fee():
    """Message fee estimation endpoint"""

    _check_block_hash(request.args)

    call = validate_request(request.data, CallL1Handler)
    fee_estimation = await state.starknet_wrapper.estimate_message_fee(call)
    return jsonify(fee_estimation)
