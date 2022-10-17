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


# without .json extension as required by load_nearby_contract
DEFAULT_ACCOUNT_PATH = os.path.join(
    __file__,
    os.pardir,
    "accounts_artifacts",
    "OpenZeppelin",
    "0.4.0b-fork",
    "Account.cairo",
    "Account.json",
)
DEFAULT_ACCOUNT_HASH_BYTES = to_bytes(
    250058203962332945652607154704986145054927159797127109843768594742871092378
)
