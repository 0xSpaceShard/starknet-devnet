"""
Global state singletone
"""

from .dump import Dumper
from .starknet_wrapper import StarknetWrapper, DevnetConfig

class State():
    """
    Stores starknet wrapper and dumper
    """
    def __init__(self):
        self.starknet_wrapper = StarknetWrapper(config=DevnetConfig())
        self.dumper = Dumper(self.starknet_wrapper)

    def __set_starknet_wrapper(self, starknet_wrapper: StarknetWrapper):
        """Sets starknet wrapper and creates new instance of dumper"""
        self.starknet_wrapper = starknet_wrapper
        self.dumper = Dumper(starknet_wrapper)

    async def reset(self, config: DevnetConfig = None):
        """Reset the starknet wrapper and dumper instances"""
        previous_config = self.starknet_wrapper.config
        self.__set_starknet_wrapper(StarknetWrapper(config=config or previous_config))
        await self.starknet_wrapper.initialize()

    def load(self, load_path: str):
        """Loads starknet wrapper from path"""
        self.__set_starknet_wrapper(StarknetWrapper.load(load_path))

state = State()
