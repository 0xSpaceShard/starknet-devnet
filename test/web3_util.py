"""Util functions for invoking and calling Web3 contracts"""

from typing import List

from web3 import Web3
from web3.contract import Contract
from web3.contract.contract import ContractFunction


def web3_deploy(web3: Web3, contract: Contract, *inputs):
    """Deploys a Solidity contract"""
    abi = contract["abi"]
    bytecode = contract["bytecode"]
    contract = web3.eth.contract(abi=abi, bytecode=bytecode)
    tx_hash = contract.constructor(*inputs).transact()
    tx_receipt = web3.eth.wait_for_transaction_receipt(tx_hash)
    return web3.eth.contract(address=tx_receipt.contractAddress, abi=abi)


def web3_transact(
    web3: Web3,
    function_name: str,
    contract: Contract,
    function_args: List,
    value=0,
):
    """Invokes a function in a Web3 contract. Argument `value` refers to msg.value (paid amount)."""

    contract_function: ContractFunction = contract.get_function_by_name(function_name)
    tx_hash = contract_function(*function_args).transact({"value": value})
    web3.eth.wait_for_transaction_receipt(tx_hash)

    return tx_hash


def web3_call(function_name: str, contract: Contract, *inputs):
    """Calls a function in a Web3 contract"""

    value = contract.get_function_by_name(function_name)(*inputs).call()
    return value
