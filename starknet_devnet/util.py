"""
Utility functions used across the project.
"""

from dataclasses import dataclass
import os
from typing import List, Dict, Union

from starkware.starkware_utils.error_handling import StarkException
from starkware.starknet.testing.contract import StarknetContract
from starkware.starknet.business_logic.execution.objects import CallInfo
from starkware.starknet.business_logic.state.state import CarriedState
from starkware.starknet.services.api.feeder_gateway.response_objects import (
    BlockStateUpdate,
    StateDiff,
    StorageEntry,
    DeployedContract,
)


def custom_int(arg: str) -> int:
    """
    Converts the argument to an integer.
    Conversion base is 16 if `arg` starts with `0x`, otherwise `10`.
    """
    base = 16 if arg.startswith("0x") else 10
    return int(arg, base)


def fixed_length_hex(arg: int) -> str:
    """
    Converts the int input to a hex output of fixed length
    """
    return f"0x{arg:064x}"


@dataclass
class Uint256:
    """Abstraction of Uint256 type"""

    low: int
    high: int

    def to_felt(self) -> int:
        """Converts to felt."""
        return (self.high << 128) + self.low

    @staticmethod
    def from_felt(felt: int) -> "Uint256":
        """Converts felt to Uint256"""
        return Uint256(low=felt & ((1 << 128) - 1), high=felt >> 128)


class StarknetDevnetException(StarkException):
    """
    Exception raised across the project.
    Indicates the raised issue is devnet-related.
    """

    def __init__(self, status_code=500, code=None, message=None):
        super().__init__(code=code, message=message)
        self.status_code = status_code


@dataclass
class DummyExecutionInfo:
    """Used if tx fails, but execution info is still required."""

    def __init__(self):
        self.actual_fee = 0
        self.call_info = CallInfo.empty_for_testing()
        self.retdata = []
        self.internal_calls = []
        self.l2_to_l1_messages = []
        self.raw_events = []

    def get_sorted_events(self):
        """Return empty list"""
        return self.raw_events

    def get_sorted_l2_to_l1_messages(self):
        """Return empty list"""
        return self.l2_to_l1_messages


def enable_pickling():
    """
    Extends the `StarknetContract` class to enable pickling.
    """

    def contract_getstate(self):
        return self.__dict__

    def contract_setstate(self, state):
        self.__dict__ = state

    StarknetContract.__getstate__ = contract_getstate
    StarknetContract.__setstate__ = contract_setstate


def generate_storage_diff(
    previous_storage_updates, storage_updates
) -> List[StorageEntry]:
    """
    Returns storage diff between previous and current storage updates
    """
    storage_diff = []

    for storage_key, leaf in storage_updates.items():
        previous_leaf = (
            previous_storage_updates.get(storage_key)
            if previous_storage_updates
            else None
        )

        if previous_leaf is None or previous_leaf.value != leaf.value:
            storage_diff.append(StorageEntry(key=storage_key, value=leaf.value))

    return storage_diff


def generate_state_update(
    previous_state: CarriedState, current_state: CarriedState
) -> BlockStateUpdate:
    """
    Returns roots, deployed contracts and storage diffs between 2 states
    """
    deployed_contracts: List[DeployedContract] = []
    declared_contracts: List[int] = []
    storage_diffs: Dict[int, List[StorageEntry]] = {}

    for class_hash in current_state.contract_definitions:
        if class_hash not in previous_state.contract_definitions:
            declared_contracts.append(int.from_bytes(class_hash, byteorder="big"))

    for contract_address in current_state.contract_states:
        if contract_address not in previous_state.contract_states:
            class_hash = int.from_bytes(
                current_state.contract_states[contract_address].state.contract_hash,
                "big",
            )
            deployed_contracts.append(
                DeployedContract(address=contract_address, class_hash=class_hash)
            )
        else:
            previous_storage_updates = previous_state.contract_states[
                contract_address
            ].storage_updates
            storage_updates = current_state.contract_states[
                contract_address
            ].storage_updates
            storage_diff = generate_storage_diff(
                previous_storage_updates, storage_updates
            )

            if len(storage_diff) > 0:
                storage_diffs[contract_address] = storage_diff

    new_root = current_state.shared_state.contract_states.root
    old_root = previous_state.shared_state.contract_states.root
    state_diff = StateDiff(
        deployed_contracts=deployed_contracts,
        declared_contracts=tuple(declared_contracts),
        storage_diffs=storage_diffs,
    )

    return BlockStateUpdate(
        block_hash=None, new_root=new_root, old_root=old_root, state_diff=state_diff
    )


def to_bytes(value: Union[int, bytes]) -> bytes:
    """
    If int, convert to 32-byte big-endian bytes instance
    If bytes, return the received value
    """
    return value if isinstance(value, bytes) else value.to_bytes(32, "big")


def check_valid_dump_path(dump_path: str):
    """Checks if dump path is a directory. Raises ValueError if not."""

    dump_path_dir = os.path.dirname(dump_path)

    if not dump_path_dir:
        # dump_path is just a file, with no parent dir
        return

    if not os.path.isdir(dump_path_dir):
        raise ValueError(f"Invalid dump path: directory '{dump_path_dir}' not found.")
