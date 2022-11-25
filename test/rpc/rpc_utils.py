"""
Utilities for RPC tests
"""

from __future__ import annotations

import re
from test.account import invoke
from test.settings import APP_URL
from test.shared import (
    PREDEPLOYED_ACCOUNT_ADDRESS,
    PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
    STORAGE_CONTRACT_PATH,
)
from test.util import assert_transaction, deploy
from typing import List, Union

import requests


class BackgroundDevnetClient:
    """A thin wrapper for requests, to interact with a background devnet instance"""

    @staticmethod
    def get(endpoint: str) -> requests.Response:
        """Submit get request at given endpoint"""
        return requests.get(f"{APP_URL}{endpoint}")

    @staticmethod
    def post(endpoint: str, body: dict) -> requests.Response:
        """Submit post request with given dict in body (JSON)"""
        return requests.post(f"{APP_URL}{endpoint}", json=body)


def make_rpc_payload(method: str, params: Union[dict, list]):
    """
    Make a wrapper for rpc call
    """
    return {"jsonrpc": "2.0", "method": method, "params": params, "id": 0}


def rpc_call_background_devnet(method: str, params: Union[dict, list]):
    """
    RPC call to devnet in backgound
    """
    payload = make_rpc_payload(method, params)
    return BackgroundDevnetClient.post("/rpc", payload).json()


def rpc_call(method: str, params: Union[dict, list]) -> dict:
    """
    Make a call to the RPC endpoint
    """
    return BackgroundDevnetClient.post(
        "/rpc", body=make_rpc_payload(method, params)
    ).json()


def add_transaction(body: dict) -> dict:
    """
    Make a call to the gateway add_transaction endpoint
    """
    response = BackgroundDevnetClient.post("/gateway/add_transaction", body)
    return response.json()


def gateway_call(method: str, **kwargs):
    """
    Make a call to the gateway
    """
    response = BackgroundDevnetClient.get(
        f"/feeder_gateway/{method}?{'&'.join(f'{key}={value}&' for key, value in kwargs.items())}"
    )
    return response.json()


def get_block_with_transaction(transaction_hash: str) -> dict:
    """
    Retrieve block for given transaction
    """
    transaction = gateway_call("get_transaction", transactionHash=transaction_hash)
    assert (
        transaction["status"] != "NOT_RECEIVED"
    ), f"Transaction {transaction_hash} was not received or does not exist"
    block_number: int = transaction["block_number"]
    block = gateway_call("get_block", blockNumber=block_number)
    return block


def get_latest_block() -> dict:
    """
    Retrive the latest block
    """
    return gateway_call("get_block", blockNumber="latest")


def deploy_and_invoke_storage_contract(value: int) -> List[str]:
    """
    Deploy and invoke storage contract
    """
    deploy_dict = deploy(STORAGE_CONTRACT_PATH)
    contract_address = deploy_dict["address"]

    invoke_tx_hash = invoke(
        calls=[(contract_address, "store_value", [value])],
        account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
    )
    assert_transaction(invoke_tx_hash, "ACCEPTED_ON_L2")

    return contract_address, invoke_tx_hash


def is_felt(value: str) -> bool:
    """
    Check whether value is a Felt
    """
    return bool(re.match(r"^0x0[a-fA-F0-9]{1,63}$", value))
