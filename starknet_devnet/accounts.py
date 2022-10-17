"""
Class representing list of predefined accounts
"""

import random
import json
import sys

from typing import List
from starkware.crypto.signature.signature import private_to_stark_key
from starkware.python.utils import to_bytes
from starkware.solidity.utils import load_nearby_contract
from starkware.starknet.core.os.class_hash import compute_class_hash
from starkware.starknet.services.api.contract_class import ContractClass

from starknet_devnet.contract_class_wrapper import (
    DEFAULT_ACCOUNT_PATH,
    DEFAULT_ACCOUNT_HASH_BYTES,
    ContractClassWrapper,
)
from starknet_devnet.util import StarknetDevnetException
from .account import Account


def __load_account_class(path: str) -> ContractClass:
    """Load contract class from `path`"""
    with open(path, mode="r", encoding="utf-8") as dict_file:
        loaded_dict = json.load(dict_file)
        contract_class = ContractClass.load(loaded_dict)
        for account_method in ["__execute__", "__validate__", "__validate_declare__"]:
            if account_method not in contract_class.abi:
                # throw / exit with 1
                # TODO this could be done in config.py
        return contract_class


class Accounts:
    """Accounts wrapper"""

    list: List[Account]

    def __init__(self, starknet_wrapper):
        self.starknet_wrapper = starknet_wrapper
        self.__n_accounts = starknet_wrapper.config.accounts
        self.__initial_balance = starknet_wrapper.config.initial_balance

        if starknet_wrapper.config.account_path:
            account_class = __load_account_class(starknet_wrapper.config.account_path)
            account_class_hash_bytes = to_bytes(compute_class_hash(account_class))
        else:
            account_class = ContractClass.load(
                load_nearby_contract(DEFAULT_ACCOUNT_PATH)
            )
            account_class_hash_bytes = DEFAULT_ACCOUNT_HASH_BYTES

        self.__account_class_wrapper = ContractClassWrapper(
            contract_class=account_class, hash_bytes=account_class_hash_bytes
        )

        self.__seed = starknet_wrapper.config.seed
        if self.__seed is None:
            self.__seed = random.getrandbits(32)

        self.list = []

        self.__generate()
        if (
            starknet_wrapper.config.accounts
            and not starknet_wrapper.config.hide_predeployed_accounts
        ):
            self.__print()

    def __getitem__(self, index):
        return self.list[index]

    async def deploy(self):
        """deploy listed accounts"""
        for account in self.list:
            await account.deploy()

    def add(self, account):
        """append account to list"""
        self.list.append(account)
        return account

    def __generate(self):
        """Generates accounts without deploying them"""
        random_generator = random.Random()
        random_generator.seed(self.__seed)

        for _ in range(self.__n_accounts):
            private_key = random_generator.getrandbits(128)
            public_key = private_to_stark_key(private_key)

            self.add(
                Account(
                    self.starknet_wrapper,
                    private_key=private_key,
                    public_key=public_key,
                    initial_balance=self.__initial_balance,
                    account_class_wrapper=self.__account_class_wrapper,
                )
            )

    def __print(self):
        """stdout accounts list"""
        for idx, account in enumerate(self):
            print(f"Account #{idx}")
            print(f"Address: {hex(account.address)}")
            print(f"Public key: {hex(account.public_key)}")
            print(f"Private key: {hex(account.private_key)}\n")

        print(f"Initial balance of each account: {self.__initial_balance} WEI")
        print("Seed to replicate this account sequence:", self.__seed)
        print(
            "WARNING: Use these accounts and their keys ONLY for local testing. "
            "DO NOT use them on mainnet or other live networks because you will LOSE FUNDS.\n",
            file=sys.stderr,
        )
        sys.stdout.flush()
