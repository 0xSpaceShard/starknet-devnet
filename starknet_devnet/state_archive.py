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

    def __init__(self, min_number: int):
        # TODO handle restarts
        # TODO state dumping
        self._min_number = min_number
        self._max_number = min_number

    def store(self, number: int, state: StarknetState):
        """Store the state under the given number"""
        self._max_number += 1
        assert number == self._max_number, f"State stored under wrong number: {number}"

        self._store(number, state)

    def get(self, number: int) -> StarknetState:
        """
        Returns the state stored under `number`.
        Raises if out of range.
        """
        if number < self._min_number or number > self._max_number:
            raise StarknetDevnetException(
                code=StarknetErrorCode.OUT_OF_RANGE_BLOCK_ID,
                message=f"Cannot use block state before {self._min_number} or after {self._max_number}. Got: {number}",
            )

        return self._get(number)

    def _store(self, number: int, state: StarknetState):
        raise NotImplementedError

    def _get(self, number: int) -> StarknetState:
        raise NotImplementedError


class MemoryStateArchive(StateArchive):
    """
    Stores StarkNet states in memory
    """

    def __init__(self, min_number: int):
        super().__init__(min_number)
        self.__storage = {}

    def _store(self, number: int, state: StarknetState):
        self.__storage[number] = state  # .copy()  # TODO

    def _get(self, number: int) -> StarknetState:
        return self.__storage[number]


class DiskStateArchive(StateArchive):
    """
    Stores StarkNet states on disk
    """

    PATH = "/tmp/starknet-devnet-state.db"

    def __init__(self, min_number: int):
        super().__init__(min_number)
        with shelve.open(self.PATH, flag="n"):
            # just create the database (always a new - overwrite the old one)
            pass

    def _store(self, number: int, state: StarknetState):
        with shelve.open(self.PATH, flag="w") as storage:
            storage[str(number)] = state

    def _get(self, number: int) -> StarknetState:
        with shelve.open(self.PATH, flag="r") as storage:
            return storage[str(number)]
