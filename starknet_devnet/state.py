"""
Global state singletone
"""

from .devnet_config import DevnetConfig
from .dump import Dumper
from .starknet_wrapper import StarknetWrapper

class State():
    """
    Stores starknet wrapper and dumper
    """
    def __init__(self):
        self.set_starknet_wrapper(StarknetWrapper(DevnetConfig()))

    def set_starknet_wrapper(self, starknet_wrapper: StarknetWrapper):
        """Sets starknet wrapper and creates new instance of dumper"""
        self.starknet_wrapper = starknet_wrapper
        self.dumper = Dumper(starknet_wrapper)

    async def reset(self):
        """Reset the starknet wrapper and dumper instances"""
        previous_config = self.starknet_wrapper.config
        self.set_starknet_wrapper(StarknetWrapper(previous_config))
        await self.starknet_wrapper.initialize()

    def load(self, load_path: str):
        """Loads starknet wrapper from path"""
        self.set_starknet_wrapper(StarknetWrapper.load(load_path))

state = State()
