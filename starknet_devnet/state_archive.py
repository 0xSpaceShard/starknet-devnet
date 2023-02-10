"""
Stores StarkNet states
"""

import shelve
from starkware.starknet.testing.state import StarknetState


class StateArchive:
    """
    Stores StarkNet states
    """

    PATH = "/tmp/starknet-devnet-state.db"

    def __init__(self, origin):
        self.origin = origin
        # TODO handle restarts
        # TODO state dumping
        with shelve.open(self.PATH, flag="n"):
            # just create the database (always a new one - overwrite the old)
            pass

    def store(self, number: int, state: StarknetState):
        """Store the state under the next number"""
        with shelve.open(self.PATH, flag="w") as storage:
            storage[str(number)] = state

    def get(self, number: int):
        """Returns the state stored under `number`"""
        with shelve.open(self.PATH, flag="r") as storage:
            return storage[str(number)]
