"""
Feeder gateway routes.
"""

from flask import request, jsonify, Blueprint, Response
from marshmallow import ValidationError
from starkware.starknet.services.api.feeder_gateway.response_objects import BlockTransactionTraces
from starkware.starknet.services.api.gateway.transaction import InvokeFunction
from werkzeug.datastructures import MultiDict

from starknet_devnet.state import state
from starknet_devnet.util import StarknetDevnetException, custom_int, fixed_length_hex

feeder_gateway = Blueprint("feeder_gateway", __name__, url_prefix="/feeder_gateway")

def validate_call(data: bytes):
    """Ensure `data` is valid Starknet function call. Returns an `InvokeFunction`."""

    try:
        call_specifications = InvokeFunction.loads(data)
    except (TypeError, ValidationError) as err:
        raise StarknetDevnetException(message=f"Invalid Starknet function call: {err}", status_code=400) from err

    return call_specifications

def _check_block_hash(request_args: MultiDict):
    block_hash = request_args.get("blockHash", type=custom_int)
    if block_hash is not None:
        print("Specifying a block by its hash is not supported. All interaction is done with the latest block.")

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

@feeder_gateway.route("/is_alive", methods=["GET"])
def is_alive():
    """Health check endpoint."""
    return "Alive!!!"

@feeder_gateway.route("/get_contract_addresses", methods=["GET"])
def get_contract_addresses():
    """Endpoint that returns an object containing the addresses of key system components."""
    return "Not implemented", 501

@feeder_gateway.route("/call_contract", methods=["POST"])
async def call_contract():
    """
    Endpoint for receiving calls (not invokes) of contract functions.
    """

    call_specifications = validate_call(request.data)

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

    traces = []
    for transaction in block.transaction_receipts:
        tx_hash = hex(transaction.transaction_hash)
        trace = state.starknet_wrapper.transactions.get_transaction_trace(tx_hash)

        # expected trace is equal to response of get_transaction, but with the hash property
        trace_dict = trace.dump()
        trace_dict["transaction_hash"] = tx_hash
        traces.append(trace_dict)

    # assert correct structure
    block_transaction_traces = BlockTransactionTraces.load({ "traces": traces })

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

    contract_class = state.starknet_wrapper.contracts.get_full_contract(contract_address)

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
    """Endpoint for returning the storage identified by `key` from the contract at """
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
    transaction_status = state.starknet_wrapper.transactions.get_transaction_status(transaction_hash)
    return jsonify(transaction_status)

@feeder_gateway.route("/get_transaction", methods=["GET"])
def get_transaction():
    """
    Returns the transaction identified by the transactionHash argument in the GET request.
    """

    transaction_hash = request.args.get("transactionHash")
    transaction_info = state.starknet_wrapper.transactions.get_transaction(transaction_hash)
    return Response(response=transaction_info.dumps(), status=200, mimetype="application/json")

@feeder_gateway.route("/get_transaction_receipt", methods=["GET"])
def get_transaction_receipt():
    """
    Returns the transaction receipt identified by the transactionHash argument in the GET request.
    """

    transaction_hash = request.args.get("transactionHash")
    transaction_receipt = state.starknet_wrapper.transactions.get_transaction_receipt(transaction_hash)
    return Response(response=transaction_receipt.dumps(), status=200, mimetype="application/json")

@feeder_gateway.route("/get_transaction_trace", methods=["GET"])
def get_transaction_trace():
    """
    Returns the trace of the transaction identified by the transactionHash argument in the GET request.
    """

    transaction_hash = request.args.get("transactionHash")
    transaction_trace = state.starknet_wrapper.transactions.get_transaction_trace(transaction_hash)

    return Response(response=transaction_trace.dumps(), status=200, mimetype="application/json")

@feeder_gateway.route("/get_state_update", methods=["GET"])
def get_state_update():
    """
    Returns the status update from the block identified by the blockHash argument in the GET request.
    If no block hash was provided it will default to the last block.
    """

    block_hash = request.args.get("blockHash")
    block_number = request.args.get("blockNumber", type=custom_int)

    state_update = state.starknet_wrapper.blocks.get_state_update(block_hash=block_hash, block_number=block_number)

    if state_update is not None:
        return Response(response=state_update.dumps(), status=200, mimetype="application/json")

    return jsonify(state_update)

@feeder_gateway.route("/estimate_fee", methods=["POST"])
async def estimate_fee():
    """Returns the estimated fee for a transaction."""
    transaction = validate_call(request.data)
    fee_response = await state.starknet_wrapper.calculate_actual_fee(transaction)

    return jsonify(fee_response)
