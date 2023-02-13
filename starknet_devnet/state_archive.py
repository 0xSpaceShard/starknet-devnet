"""
Stores StarkNet states
"""

import shelve

from starkware.starknet.definitions.error_codes import StarknetErrorCode
from starkware.starknet.testing.state import StarknetState

from .util import StarknetDevnetException


class StateArchive:
    """
    Stores StarkNet states
    """

    PATH = "/tmp/starknet-devnet-state.db"

    def __init__(self, min_number: int):
        self.__min_number = min_number
        self.__max_number = min_number

        # TODO handle restarts
        # TODO state dumping
        with shelve.open(self.PATH, flag="n"):
            # just create the database (always a new - overwrite the old one)
            pass

    def store(self, number: int, state: StarknetState):
        """Store the state under the given number, incrementing the max number"""
        self.__max_number += 1
        assert number == self.__max_number, f"State stored under wrong number: {number}"

        with shelve.open(self.PATH, flag="w") as storage:
            storage[str(number)] = state

    def get(self, number: int):
        """
        Returns the state stored under `number`.
        Checks the number to prevent unnecessary reads. Raises if out of range.
        """
        if number < self.__min_number or number > self.__max_number:
            raise StarknetDevnetException(
                code=StarknetErrorCode.OUT_OF_RANGE_BLOCK_ID,
                message=f"Cannot use block state before {self.__min_number} or after {self.__max_number}. Got: {number}",
            )

        with shelve.open(self.PATH, flag="r") as storage:
            return storage[str(number)]
