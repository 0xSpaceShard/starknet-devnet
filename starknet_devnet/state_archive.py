"""
Stores Starknet states
"""

import shelve

from starkware.starknet.definitions.error_codes import StarknetErrorCode
from starkware.starknet.testing.state import StarknetState

from .util import StarknetDevnetException


class StateArchive:
    """
    Stores Starknet states
    """

    def store(self, number: int, state: StarknetState):
        """Store the state under the given number"""
        self._storage_write(number, state)

    def remove(self, number: int):
        """Remove the state under the given number"""
        self._storage_remove(number)

    def get(self, number: int) -> StarknetState:
        """
        Returns the state stored under `number`.
        Raises if out of range.
        """
        try:
            return self._storage_read(number)
        except KeyError as error:
            raise StarknetDevnetException(
                code=StarknetErrorCode.OUT_OF_RANGE_BLOCK_ID,
                message=f"State at block {number} not present",
            ) from error

    def _storage_write(self, number: int, state: StarknetState):
        raise NotImplementedError

    def _storage_read(self, number: int) -> StarknetState:
        raise NotImplementedError

    def _storage_remove(self, number: int) -> StarknetState:
        raise NotImplementedError


class MemoryStateArchive(StateArchive):
    """
    Stores Starknet states in memory
    """

    def __init__(self):
        super().__init__()
        self.__storage = {}

    def _storage_write(self, number: int, state: StarknetState):
        self.__storage[number] = state.copy()

    def _storage_remove(self, number: int):
        del self.__storage[number]

    def _storage_read(self, number: int) -> StarknetState:
        return self.__storage[number]


class DiskStateArchive(StateArchive):
    """
    Stores Starknet states on disk
    """

    PATH = "/tmp/starknet-devnet-state.db"

    def __init__(self):
        super().__init__()
        with shelve.open(self.PATH, flag="n"):
            # just create the database (always a new - overwrite the old one)
            pass

    def _storage_write(self, number: int, state: StarknetState):
        with shelve.open(self.PATH, flag="w") as storage:
            storage[str(number)] = state

    def _storage_read(self, number: int) -> StarknetState:
        with shelve.open(self.PATH, flag="r") as storage:
            return storage[str(number)]

    def _storage_remove(self, number: int) -> StarknetState:
        with shelve.open(self.PATH, flag="w") as storage:
            del storage[str(number)]
