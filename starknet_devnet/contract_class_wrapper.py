"""Starknet ContractClass wrapper utilities"""

from dataclasses import dataclass
import os

from starkware.python.utils import to_bytes
from starkware.starknet.services.api.contract_class import ContractClass


@dataclass
class ContractClassWrapper:
    """Wrapper of ContractClass"""

    contract_class: ContractClass
    hash_bytes: bytes


DEFAULT_ACCOUNT_PATH = os.path.abspath(
    os.path.join(
        __file__,
        os.pardir,
        "accounts_artifacts",
        "OpenZeppelin",
        "0.5.0",
        "Account.cairo",
        "Account.json",
    )
)
DEFAULT_ACCOUNT_HASH_BYTES = to_bytes(
    2308850740939678659398575035812067402979543458539300415910488838841673668983
)
