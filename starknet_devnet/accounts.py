"""
Class representing list of predefined accounts
"""

import random
import sys

from typing import List
from starkware.crypto.signature.signature import private_to_stark_key
from .account import Account


class Accounts:
    """Accounts wrapper"""

    list: List[Account]

    def __init__(self, starknet_wrapper):
        self.starknet_wrapper = starknet_wrapper
        self.__n_accounts = starknet_wrapper.config.accounts
        self.__initial_balance = starknet_wrapper.config.initial_balance
        self.__seed = starknet_wrapper.config.seed
        self.list = []

        self.__generate()
        if starknet_wrapper.config.accounts:
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
        self.__initial_balance = self.__initial_balance

        random_generator.seed(
            self.__seed if self.__seed is not None else random_generator.getrandbits(32)
        )

        for _ in range(self.__n_accounts):
            private_key = random_generator.getrandbits(128)
            public_key = private_to_stark_key(private_key)

            self.add(
                Account(
                    self.starknet_wrapper,
                    private_key=private_key,
                    public_key=public_key,
                    initial_balance=self.__initial_balance,
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
