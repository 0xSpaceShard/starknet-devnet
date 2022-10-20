"""
This module introduces `LiteInternalDeploy`, optimized lite-mode version of InternalDeploy.
"""
from typing import List

from starkware.starknet.business_logic.transaction.objects import InternalDeploy
from starkware.starknet.services.api.gateway.transaction import (
    Deploy,
    Transaction,
    EverestTransaction,
)
from starkware.starknet.services.api.contract_class import ContractClass
from starkware.starknet.business_logic.utils import verify_version
from starkware.starknet.core.os.contract_address.contract_address import (
    calculate_contract_address_from_hash,
)
from starkware.starknet.core.os.class_hash import compute_class_hash
from starkware.starknet.business_logic.transaction.objects import InternalTransaction
from starkware.python.utils import to_bytes

from starknet_devnet.constants import OLD_SUPPORTED_VERSIONS

# pylint: disable=too-many-ancestors, arguments-renamed, too-many-arguments
class LiteInternalDeploy(InternalDeploy):
    """
    The lite version of InternalDeploy which avoid transaction hash a calculation in deploy.
    """

    @classmethod
    def _specific_from_external(
        cls,
        external_tx: Transaction,
        tx_number: int,
    ) -> "LiteInternalDeploy":
        """
        Lite version of _specific_from_external method.
        """
        assert isinstance(external_tx, Deploy)
        return cls.lite_create(
            contract_address_salt=external_tx.contract_address_salt,
            contract_class=external_tx.contract_definition,
            constructor_calldata=external_tx.constructor_calldata,
            version=external_tx.version,
            tx_number=tx_number,
        )

    @classmethod
    def from_external(
        cls, external_tx: EverestTransaction, tx_number: int
    ) -> InternalTransaction:
        """
        Returns an internal transaction genearated based on an external one.
        """
        # Downcast arguments to application-specific types.
        assert isinstance(external_tx, Transaction)

        internal_cls = LiteInternalDeploy.external_to_internal_cls.get(
            type(external_tx)
        )
        if internal_cls is None:
            raise NotImplementedError(
                f"Unsupported transaction type {type(external_tx).__name__}."
            )

        return LiteInternalDeploy._specific_from_external(
            external_tx=external_tx, tx_number=tx_number
        )

    @classmethod
    def lite_create(
        cls,
        contract_address_salt: int,
        contract_class: ContractClass,
        constructor_calldata: List[int],
        version: int,
        tx_number: int,
    ):
        """
        Lite version of create method without hash a calculation.
        """
        verify_version(
            version=version,
            only_query=False,
            old_supported_versions=OLD_SUPPORTED_VERSIONS,
        )
        class_hash = compute_class_hash(contract_class=contract_class)
        contract_address = calculate_contract_address_from_hash(
            salt=contract_address_salt,
            class_hash=class_hash,
            constructor_calldata=constructor_calldata,
            deployer_address=0,
        )

        return cls(
            contract_address=contract_address,
            contract_address_salt=contract_address_salt,
            contract_hash=to_bytes(class_hash),
            constructor_calldata=constructor_calldata,
            version=version,
            hash_value=tx_number,
        )
