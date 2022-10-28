"""Forked state"""

from starkware.starknet.business_logic.state.state_api import StateReader
from starkware.storage.dict_storage import DictStorage
from starkware.starknet.business_logic.fact_state.state import SharedState
from starkware.starknet.business_logic.fact_state.patricia_state import (
    PatriciaStateReader,
)
from starkware.starknet.services.api.contract_class import ContractClass
from starkware.storage.storage import FactFetchingContext
from starkware.cairo.lang.vm.crypto import pedersen_hash_func
from starkware.starknet.definitions.general_config import StarknetGeneralConfig


class ForkedStateReader(StateReader):
    """State with a fallback to a forked origin"""

    def __init__(self, inner_state_reader: StateReader):
        self.__inner_state_reader = inner_state_reader

    @classmethod
    async def create(cls, general_config: StarknetGeneralConfig):
        """Create an instance of this class"""

        ffc = FactFetchingContext(storage=DictStorage(), hash_func=pedersen_hash_func)
        empty_shared_state = await SharedState.empty(
            ffc=ffc, general_config=general_config
        )
        inner_state_reader = PatriciaStateReader(
            global_state_root=empty_shared_state.contract_states, ffc=ffc
        )

        return cls(inner_state_reader)

    async def get_contract_class(self, class_hash: bytes) -> ContractClass:
        # TODO add wrapper in case not found
        return await self.__inner_state_reader.get_contract_class(class_hash)

    async def _get_raw_contract_class(self, class_hash: bytes) -> bytes:
        # pylint: disable=protected-access
        return await self.__inner_state_reader._get_raw_contract_class(class_hash)

    async def get_class_hash_at(self, contract_address: int) -> bytes:
        return await self.__inner_state_reader.get_class_hash_at(contract_address)

    async def get_nonce_at(self, contract_address: int) -> int:
        return await self.__inner_state_reader.get_nonce_at(contract_address)

    async def get_storage_at(self, contract_address: int, key: int) -> int:
        return await self.__inner_state_reader.get_storage_at(contract_address, key)
