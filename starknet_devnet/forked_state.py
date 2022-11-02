"""Forked state"""

from services.external_api.client import (
    RetryConfig,
)  # TODO add dependency in pyproject.toml
from starkware.cairo.lang.vm.crypto import pedersen_hash_func
from starkware.starknet.business_logic.state.state_api import StateReader
from starkware.storage.dict_storage import DictStorage
from starkware.starknet.business_logic.fact_state.patricia_state import (
    PatriciaStateReader,
)
from starkware.starknet.business_logic.fact_state.state import SharedState
from starkware.starknet.definitions.general_config import StarknetGeneralConfig
from starkware.starknet.services.api.contract_class import ContractClass
from starkware.starknet.services.api.feeder_gateway.feeder_gateway_client import (
    FeederGatewayClient,
)
from starkware.storage.storage import FactFetchingContext


class ForkedStateReader(StateReader):
    """State with a fallback to a forked origin"""

    def __init__(
        self,
        inner_state_reader: StateReader,
        feeder_gateway_client: FeederGatewayClient,
        block_number: int,
    ):
        self.__inner_state_reader = inner_state_reader
        self.__feeder_gateway_client = feeder_gateway_client
        self.__block_number = block_number

    @classmethod
    async def create(
        cls,
        general_config: StarknetGeneralConfig,
        feeder_gateway_url: str,
        block_id: int,
    ):
        """Create an instance of this class"""

        ffc = FactFetchingContext(storage=DictStorage(), hash_func=pedersen_hash_func)
        empty_shared_state = await SharedState.empty(
            ffc=ffc, general_config=general_config
        )
        inner_state_reader = PatriciaStateReader(
            global_state_root=empty_shared_state.contract_states, ffc=ffc
        )

        feeder_gateway_client = FeederGatewayClient(
            url=feeder_gateway_url,
            retry_config=RetryConfig(n_retries=1),
        )

        block = await feeder_gateway_client.get_block(block_number=block_id)

        return cls(inner_state_reader, feeder_gateway_client, block.block_number)

    async def get_contract_class(self, class_hash: bytes) -> ContractClass:
        try:
            return await self.__inner_state_reader.get_contract_class(class_hash)
        except:  # TODO type
            class_hash_hex = class_hash.hex()
            contract_class_dict = await self.__feeder_gateway_client.get_class_by_hash(
                class_hash_hex
            )
            return ContractClass.load(contract_class_dict)

    async def _get_raw_contract_class(self, class_hash: bytes) -> bytes:
        # TODO default to origin
        # pylint: disable=protected-access
        return await self.__inner_state_reader._get_raw_contract_class(class_hash)

    async def get_class_hash_at(self, contract_address: int) -> bytes:
        try:
            return await self.__inner_state_reader.get_class_hash_at(contract_address)
        except:  # TODO type
            class_hash_hex = await self.__feeder_gateway_client.get_class_hash_at(
                contract_address
            )
            return bytes.fromhex(class_hash_hex)

    async def get_nonce_at(self, contract_address: int) -> int:
        try:
            return await self.__inner_state_reader.get_nonce_at(contract_address)
        except:  # TODO type
            return await self.__feeder_gateway_client.get_nonce(contract_address)

    async def get_storage_at(self, contract_address: int, key: int) -> int:
        try:
            return await self.__inner_state_reader.get_storage_at(contract_address, key)
        except Exception as err:  # TODO type
            print("DEBUG err in forked state reader", type(err))
            storage_hex = await self.__feeder_gateway_client.get_storage_at(
                contract_address=contract_address,
                key=key,
            )

            return int(storage_hex, 16)
