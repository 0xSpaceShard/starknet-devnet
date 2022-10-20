"""
Contains classes that provide the abstraction of L2 blockchain.
"""

from starkware.starknet.definitions.error_codes import StarknetErrorCode
from starkware.starknet.services.api.contract_class import ContractClass
from starkware.starknet.services.api.feeder_gateway.response_objects import (
    TransactionStatus,
    TransactionInfo,
    TransactionReceipt,
    TransactionTrace,
    StarknetBlock,
)

from starknet_devnet.util import StarknetDevnetException


class Origin:
    """
    Abstraction of an L2 blockchain.
    """

    def get_transaction_status(self, transaction_hash: str):
        """Returns the status of the transaction."""
        raise NotImplementedError

    def get_transaction(self, transaction_hash: str) -> TransactionInfo:
        """Returns the transaction object."""
        raise NotImplementedError

    def get_transaction_receipt(self, transaction_hash: str) -> TransactionReceipt:
        """Returns the transaction receipt object."""
        raise NotImplementedError

    def get_transaction_trace(self, transaction_hash: str) -> TransactionTrace:
        """Returns the transaction trace object."""
        raise NotImplementedError

    def get_block_by_hash(self, block_hash: str) -> StarknetBlock:
        """Returns the block identified with either its hash."""
        raise NotImplementedError

    def get_block_by_number(self, block_number: int) -> StarknetBlock:
        """Returns the block identified with either its number or the latest block if no number provided."""
        raise NotImplementedError

    def get_code(self, contract_address: int) -> dict:
        """Returns the code of the contract."""
        raise NotImplementedError

    def get_full_contract(self, contract_address: int) -> dict:
        """Returns the contract class"""
        raise NotImplementedError

    def get_class_by_hash(self, class_hash: int) -> ContractClass:
        """Returns the contract class from its hash"""
        raise NotImplementedError

    def get_class_hash_at(self, contract_address: int) -> int:
        """Returns the class hash at the provided address"""
        raise NotImplementedError

    def get_storage_at(self, contract_address: int, key: int) -> str:
        """Returns the storage identified with `key` at `contract_address`."""
        raise NotImplementedError

    def get_number_of_blocks(self):
        """Returns the number of blocks stored so far"""
        raise NotImplementedError

    def get_state_update(
        self, block_hash: str = None, block_number: int = None
    ) -> dict or None:
        """
        Returns the state update for provided block hash or block number.
        If none are provided return the last state update
        """
        raise NotImplementedError


class NullOrigin(Origin):
    """
    A default class to comply with the Origin interface.
    """

    def get_transaction_status(self, transaction_hash: str):
        return {"tx_status": TransactionStatus.NOT_RECEIVED.name}

    def get_transaction(self, transaction_hash: str) -> TransactionInfo:
        return TransactionInfo.create(
            status=TransactionStatus.NOT_RECEIVED,
        )

    def get_transaction_receipt(self, transaction_hash: str) -> TransactionReceipt:
        return TransactionReceipt(
            status=TransactionStatus.NOT_RECEIVED,
            transaction_hash=int(transaction_hash, 16),
            events=[],
            l2_to_l1_messages=[],
            block_hash=None,
            block_number=None,
            transaction_index=None,
            execution_resources=None,
            actual_fee=None,
            transaction_failure_reason=None,
            l1_to_l2_consumed_message=None,
        )

    def get_transaction_trace(self, transaction_hash: str):
        tx_hash_int = int(transaction_hash, 16)
        message = f"Transaction corresponding to hash {tx_hash_int} is not found."
        raise StarknetDevnetException(
            code=StarknetErrorCode.INVALID_TRANSACTION_HASH, message=message
        )

    def get_block_by_hash(self, block_hash: str):
        message = f"Block hash not found; got: {block_hash}."
        raise StarknetDevnetException(
            code=StarknetErrorCode.BLOCK_NOT_FOUND, message=message
        )

    def get_block_by_number(self, block_number: int):
        message = "Requested the latest block, but there are no blocks so far."
        raise StarknetDevnetException(
            code=StarknetErrorCode.BLOCK_NOT_FOUND, message=message
        )

    def get_code(self, contract_address: int):
        return {"abi": {}, "bytecode": []}

    def get_full_contract(self, contract_address: int) -> dict:
        return {"abi": {}, "entry_points_by_type": {}, "program": {}}

    def get_class_by_hash(self, class_hash: int) -> ContractClass:
        message = f"Class with hash {hex(class_hash)} is not declared."
        raise StarknetDevnetException(
            code=StarknetErrorCode.UNDECLARED_CLASS, message=message
        )

    def get_class_hash_at(self, contract_address: int) -> int:
        message = f"Contract with address {hex(contract_address)} is not deployed."
        raise StarknetDevnetException(
            code=StarknetErrorCode.UNINITIALIZED_CONTRACT, message=message
        )

    def get_storage_at(self, contract_address: int, key: int) -> str:
        return hex(0)

    def get_number_of_blocks(self):
        return 0

    def get_state_update(
        self, block_hash: str = None, block_number: int = None
    ) -> dict or None:
        if block_hash:
            error_message = (
                f"No state updates saved for the provided block hash {block_hash}"
            )
            raise StarknetDevnetException(
                code=StarknetErrorCode.BLOCK_NOT_FOUND, message=error_message
            )

        if block_number is not None:
            error_message = (
                f"No state updates saved for the provided block number {block_number}"
            )
            raise StarknetDevnetException(
                code=StarknetErrorCode.BLOCK_NOT_FOUND, message=error_message
            )


class ForkedOrigin(Origin):
    """
    Abstracts an origin that the devnet was forked from.
    """

    def __init__(self, url):
        self.url = url
        self.number_of_blocks = ...

    def get_transaction_status(self, transaction_hash: str):
        raise NotImplementedError

    def get_transaction(self, transaction_hash: str):
        raise NotImplementedError

    def get_transaction_trace(self, transaction_hash: str):
        raise NotImplementedError

    def get_block_by_hash(self, block_hash: str):
        raise NotImplementedError

    def get_block_by_number(self, block_number: int):
        raise NotImplementedError

    def get_code(self, contract_address: int) -> dict:
        raise NotImplementedError

    def get_full_contract(self, contract_address: int) -> dict:
        raise NotImplementedError

    def get_class_by_hash(self, class_hash: int) -> ContractClass:
        raise NotImplementedError

    def get_class_hash_at(self, contract_address: int) -> int:
        raise NotImplementedError

    def get_storage_at(self, contract_address: int, key: int) -> str:
        raise NotImplementedError

    def get_number_of_blocks(self):
        return self.number_of_blocks

    def get_state_update(
        self, block_hash: str = None, block_number: int = None
    ) -> dict or None:
        raise NotImplementedError
