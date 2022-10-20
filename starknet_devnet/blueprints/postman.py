"""
Postman routes.
"""

import json

from flask import Blueprint, jsonify, request
from starkware.starkware_utils.error_handling import StarkErrorCode

from starknet_devnet.state import state
from starknet_devnet.util import StarknetDevnetException

postman = Blueprint("postman", __name__, url_prefix="/postman")


def validate_load_messaging_contract(request_dict: dict):
    """Ensure `data` is valid Starknet function call. Returns an `InvokeFunction`."""

    network_url = request_dict.get("networkUrl")
    if network_url is None:
        error_message = "L1 network or StarknetMessaging contract address not specified"
        raise StarknetDevnetException(
            code=StarkErrorCode.MALFORMED_REQUEST,
            message=error_message,
            status_code=400,
        )

    return network_url


@postman.route("/load_l1_messaging_contract", methods=["POST"])
async def load_l1_messaging_contract():
    """
    Loads a MockStarknetMessaging contract. If one is already deployed in the L1 network specified by the networkUrl argument,
    in the address specified in the address argument in the POST body, it is used, otherwise a new one will be deployed.
    The networkId argument is used to check if a local testnet instance or a public testnet should be used.
    """

    request_dict = json.loads(request.data.decode("utf-8"))
    network_url = validate_load_messaging_contract(request_dict)
    contract_address = request_dict.get("address")
    network_id = request_dict.get("networkId")

    result_dict = await state.starknet_wrapper.load_messaging_contract_in_l1(
        network_url, contract_address, network_id
    )
    return jsonify(result_dict)


@postman.route("/flush", methods=["POST"])
async def flush():
    """
    Handles all pending L1 <> L2 messages and sends them to the other layer
    """

    result_dict = await state.starknet_wrapper.postman_flush()
    return jsonify(result_dict)
