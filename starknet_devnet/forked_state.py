"""Forked state"""

from services.external_api.client import (
    RetryConfig,
)  # TODO add dependency in pyproject.toml
from starkware.python.utils import to_bytes
from starkware.starknet.business_logic.state.state_api import StateReader
from starkware.starknet.services.api.contract_class import ContractClass
from starkware.starknet.services.api.feeder_gateway.feeder_gateway_client import (
    FeederGatewayClient,
)


class ForkedStateReader(StateReader):
    """State with a fallback to a forked origin"""

    def __init__(
        self,
        feeder_gateway_client: FeederGatewayClient,
        block_number: int,
    ):
        self.__feeder_gateway_client = feeder_gateway_client
        self.__block_number = block_number

    @classmethod
    async def create(
        cls,
        feeder_gateway_url: str,
        block_id: int,
    ):
        """Create an instance of this class"""

        feeder_gateway_client = FeederGatewayClient(
            url=feeder_gateway_url,
            retry_config=RetryConfig(n_retries=1),
        )

        # TODO wasn't this already done earlier?
        block = await feeder_gateway_client.get_block(block_number=block_id)

        return cls(feeder_gateway_client, block.block_number)

    async def get_contract_class(self, class_hash: bytes) -> ContractClass:
        class_hash_hex = class_hash.hex()
        contract_class_dict = await self.__feeder_gateway_client.get_class_by_hash(
            class_hash_hex
        )
        return ContractClass.load(contract_class_dict)

    async def _get_raw_contract_class(self, class_hash: bytes) -> bytes:
        raise NotImplementedError

    async def get_class_hash_at(self, contract_address: int) -> bytes:
        class_hash_hex = await self.__feeder_gateway_client.get_class_hash_at(
            contract_address
        )
        return to_bytes(int(class_hash_hex, 16))

    async def get_nonce_at(self, contract_address: int) -> int:
        return await self.__feeder_gateway_client.get_nonce(contract_address)

    async def get_storage_at(self, contract_address: int, key: int) -> int:
        storage_hex = await self.__feeder_gateway_client.get_storage_at(
            contract_address, key
        )
        return int(storage_hex, 16)
