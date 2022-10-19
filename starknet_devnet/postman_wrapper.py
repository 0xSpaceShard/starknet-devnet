"""
This module wraps the usage of Postman for L1 <> L2 interaction.
"""
import json

from abc import ABC, abstractmethod
from web3 import HTTPProvider, Web3
from web3.middleware import geth_poa_middleware

from starkware.solidity.utils import load_nearby_contract
from starkware.starknet.definitions.error_codes import StarknetErrorCode
from starkware.starknet.testing.postman import Postman
from starkware.starknet.testing.starknet import Starknet
from starkware.eth.eth_test_utils import EthAccount, EthContract

from .constants import L1_MESSAGE_CANCELLATION_DELAY, TIMEOUT_FOR_WEB3_REQUESTS
from .util import fixed_length_hex, StarknetDevnetException


class DevnetL1L2:
    """
    This class is used to interact with the L1 <> L2 interaction.
    """

    def __init__(self) -> None:
        self.__l1_provider = None
        self.__postman_wrapper = None

    def __parse_l1_l2_messages(self, l1_raw_messages, l2_raw_messages) -> dict:
        """Converts some of the values in the dictionaries from integer to hex and keys to snake_case."""

        for message in l1_raw_messages:
            message["args"]["selector"] = hex(message["args"]["selector"])
            message["args"]["to_address"] = fixed_length_hex(
                message["args"].pop("toAddress")
            )  # L2 addresses need the leading 0
            message["args"]["from_address"] = message["args"].pop("fromAddress")
            message["args"]["payload"] = [
                hex(val) for val in message["args"]["payload"]
            ]

            # change case to snake_case
            message["transaction_hash"] = message.pop("transactionHash")
            message["block_hash"] = message.pop("blockHash")
            message["block_number"] = message.pop("blockNumber")
            message["transaction_index"] = message.pop("transactionIndex")
            message["log_index"] = message.pop("logIndex")

        l2_messages = []
        for message in l2_raw_messages:
            new_message = {
                "from_address": fixed_length_hex(
                    message.from_address
                ),  # L2 addresses need the leading 0
                "payload": [hex(val) for val in message.payload],
                "to_address": hex(message.to_address),
            }
            l2_messages.append(new_message)

        return {
            "l1_provider": self.__l1_provider,
            "consumed_messages": {"from_l1": l1_raw_messages, "from_l2": l2_messages},
        }

    def load_l1_messaging_contract(
        self,
        starknet: Starknet,
        network_url: str,
        contract_address: str,
        network_id: str,
    ) -> dict:
        """Creates a Postman Wrapper instance and loads an already deployed Messaging contract in the L1 network"""

        # If no L1 network ID provided, will use a local testnet instance
        if network_id is None or network_id == "local":
            try:
                starknet.state.l2_to_l1_messages_log.clear()
                self.__postman_wrapper = LocalPostmanWrapper(network_url)
                self.__postman_wrapper.load_mock_messaging_contract_in_l1(
                    starknet, contract_address
                )
            except Exception as error:
                message = f"""Unable to load the Starknet Messaging contract in a local testnet instance.
Make sure you have a local testnet instance running at the provided network url ({network_url}),
and that the Messaging Contract is deployed at the provided address ({contract_address})."""
                raise StarknetDevnetException(
                    code=StarknetErrorCode.UNEXPECTED_FAILURE, message=message
                ) from error
        else:
            message = "L1 interaction is only usable with a local running local testnet instance."
            raise StarknetDevnetException(
                code=StarknetErrorCode.UNEXPECTED_FAILURE, message=message
            )

        self.__l1_provider = network_url

        return {
            "l1_provider": network_url,
            "address": self.__postman_wrapper.mock_starknet_messaging_contract.address,
        }

    async def flush(self, state) -> dict:
        """Handles all pending L1 <> L2 messages and sends them to the other layer."""

        if self.__postman_wrapper is None:
            return {}

        postman = self.__postman_wrapper.postman

        l1_to_l2_messages = json.loads(
            Web3.toJSON(
                self.__postman_wrapper.l1_to_l2_message_filter.get_new_entries()
            )
        )
        l2_to_l1_messages = state.l2_to_l1_messages_log[
            postman.n_consumed_l2_to_l1_messages :
        ]

        await self.__postman_wrapper.flush()

        return self.__parse_l1_l2_messages(l1_to_l2_messages, l2_to_l1_messages)


class PostmanWrapper(ABC):
    """Postman Wrapper base class"""

    @abstractmethod
    def __init__(self):
        self.postman: Postman = None
        self.web3: Web3 = None
        self.mock_starknet_messaging_contract: EthContract = None
        self.eth_account: EthAccount = None
        self.l1_to_l2_message_filter = None

    @abstractmethod
    def load_mock_messaging_contract_in_l1(self, starknet, contract_address):
        """Retrieves the Mock Messaging contract deployed in an L1 network"""

    async def flush(self):
        """Handles the L1 <> L2 message exchange"""
        await self.postman.flush()


class LocalPostmanWrapper(PostmanWrapper):
    """Wrapper of Postman usage on a local testnet instantiated using a local testnet"""

    def __init__(self, network_url: str):
        super().__init__()
        request_kwargs = {"timeout": TIMEOUT_FOR_WEB3_REQUESTS}
        self.web3 = Web3(HTTPProvider(network_url, request_kwargs=request_kwargs))
        self.web3.middleware_onion.inject(geth_poa_middleware, layer=0)
        self.eth_account = EthAccount(self.web3, self.web3.eth.accounts[0])

    def load_mock_messaging_contract_in_l1(self, starknet, contract_address):
        if contract_address is None:
            self.mock_starknet_messaging_contract = self.eth_account.deploy(
                load_nearby_contract("MockStarknetMessaging"),
                L1_MESSAGE_CANCELLATION_DELAY,
            )
        else:
            address = Web3.toChecksumAddress(contract_address)
            contract_json = load_nearby_contract("MockStarknetMessaging")
            abi = contract_json["abi"]
            w3_contract = self.web3.eth.contract(abi=abi, address=address)
            self.mock_starknet_messaging_contract = EthContract(
                self.web3, address, w3_contract, abi, self.eth_account
            )

        self.postman = Postman(self.mock_starknet_messaging_contract, starknet)
        self.l1_to_l2_message_filter = self.mock_starknet_messaging_contract.w3_contract.events.LogMessageToL2.createFilter(
            fromBlock="latest"
        )
