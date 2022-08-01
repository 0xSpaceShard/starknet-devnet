"""
Utility functions used across the project.
"""

import argparse
from dataclasses import dataclass
from enum import Enum, auto
import os
import sys
from typing import List, Dict, Union

from starkware.starkware_utils.error_handling import StarkException
from starkware.starknet.testing.contract import StarknetContract
from starkware.starknet.business_logic.state.state import CarriedState
from starkware.starknet.services.api.feeder_gateway.response_objects import (
    BlockStateUpdate, StateDiff, StorageEntry, DeployedContract
)

from . import __version__
from .constants import (
    DEFAULT_ACCOUNTS,
    DEFAULT_GAS_PRICE,
    DEFAULT_HOST,
    DEFAULT_INITIAL_BALANCE,
    DEFAULT_PORT
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
        return Uint256(
            low=felt & ((1 << 128) - 1),
            high=felt >> 128
        )

# Uncomment this once fork support is added
# def _fork_url(name: str):
#     """
#     Return the URL corresponding to the provided name.
#     If it's not one of predefined names, assumes it is already a URL.
#     """
#     if name in ["alpha", "alpha-goerli"]:
#         return "https://alpha4.starknet.io"
#     if name == "alpha-mainnet":
#         return "https://alpha-mainnet.starknet.io"
#     # otherwise a URL; perhaps check validity
#     return name

class DumpOn(Enum):
    """Enumerate possible dumping frequencies."""
    EXIT = auto()
    TRANSACTION = auto()

DUMP_ON_OPTIONS = [e.name.lower() for e in DumpOn]
DUMP_ON_OPTIONS_STRINGIFIED = ", ".join(DUMP_ON_OPTIONS)

def parse_dump_on(option: str):
    """Parse dumping frequency option."""
    if option in DUMP_ON_OPTIONS:
        return DumpOn[option.upper()]
    sys.exit(f"Error: Invalid --dump-on option: {option}. Valid options: {DUMP_ON_OPTIONS_STRINGIFIED}")

class NonNegativeAction(argparse.Action):
    """
    Action for parsing the non negative int argument.
    """
    def __call__(self, parser, namespace, values, option_string=None):
        value = int(values)

        if value < 0:
            parser.error(f"{option_string} must be a positive integer.")

        setattr(namespace, self.dest, value)

def parse_args():
    """
    Parses CLI arguments.
    """
    parser = argparse.ArgumentParser(description="Run a local instance of Starknet Devnet")
    parser.add_argument(
        "-v", "--version",
        help="Print the version",
        action="version",
        version=__version__
    )
    parser.add_argument(
        "--host",
        help=f"Specify the address to listen at; defaults to {DEFAULT_HOST} " +
             "(use the address the program outputs on start)",
        default=DEFAULT_HOST
    )
    parser.add_argument(
        "--port", "-p",
        type=int,
        help=f"Specify the port to listen at; defaults to {DEFAULT_PORT}",
        default=DEFAULT_PORT
    )
    parser.add_argument(
        "--load-path",
        help="Specify the path from which the state is loaded on startup"
    )
    parser.add_argument(
        "--dump-path",
        help="Specify the path to dump to"
    )
    parser.add_argument(
        "--dump-on",
        help=f"Specify when to dump; can dump on: {DUMP_ON_OPTIONS_STRINGIFIED}",
        type=parse_dump_on
    )
    parser.add_argument(
        "--lite-mode",
        action='store_true',
        help="Applies all lite-mode-* optimizations by disabling some features."
    )
    parser.add_argument(
        "--lite-mode-block-hash",
        action='store_true',
        help="Disables block hash calculation"
    )
    parser.add_argument(
        "--lite-mode-deploy-hash",
        action='store_true',
        help="Disables deploy tx hash calculation"
    )
    parser.add_argument(
        "--accounts",
        type=int,
        help=f"Specify the number of accounts to be predeployed; defaults to {DEFAULT_ACCOUNTS}",
        default=DEFAULT_ACCOUNTS
    )
    parser.add_argument(
        "--initial-balance", "-e",
        type=int,
        help="Specify the initial balance of accounts to be predeployed; " +
             f"defaults to {DEFAULT_INITIAL_BALANCE:g}",
        default=DEFAULT_INITIAL_BALANCE
    )
    parser.add_argument(
        "--seed",
        type=int,
        help="Specify the seed for randomness of accounts to be predeployed"
    )
    parser.add_argument(
        "--start-time",
        action=NonNegativeAction,
        help="Specify the start time of the genesis block in Unix time seconds"
    )
    parser.add_argument(
        "--gas-price", "-g",
        action=NonNegativeAction,
        default=DEFAULT_GAS_PRICE,
        help="Specify the gas price in wei per gas unit; " +
             f"defaults to {DEFAULT_GAS_PRICE:g}"
    )
    # Uncomment this once fork support is added
    # parser.add_argument(
    #     "--fork", "-f",
    #     type=_fork_url,
    #     help="Specify the network to fork: can be a URL (e.g. https://alpha-mainnet.starknet.io) " +
    #          "or network name (alpha or alpha-mainnet)",
    # )

    args = parser.parse_args()
    if args.dump_on and not args.dump_path:
        sys.exit("Error: --dump-path required if --dump-on present")

    return args

class StarknetDevnetException(StarkException):
    """
    Exception raised across the project.
    Indicates the raised issue is devnet-related.
    """
    def __init__(self, status_code=500, code=None, message=None):
        super().__init__(code=code, message=message)
        self.status_code = status_code

@dataclass
class DummyCallInfo:
    """Used temporarily until contracts received from starknet.deploy include their own execution_info.call_info"""
    def __init__(self):
        self.execution_resources = None
        self.contract_address = None
        self.events = []
        self.internal_calls = []

@dataclass
class DummyExecutionInfo:
    """Used temporarily until contracts received from starknet.deploy include their own execution_info."""
    def __init__(self):
        self.actual_fee = 0
        self.call_info = DummyCallInfo()
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

def generate_storage_diff(previous_storage_updates, storage_updates) -> List[StorageEntry]:
    """
    Returns storage diff between previous and current storage updates
    """
    storage_diff = []

    for storage_key, leaf in storage_updates.items():
        previous_leaf = previous_storage_updates.get(storage_key) if previous_storage_updates else None

        if previous_leaf is None or previous_leaf.value != leaf.value:
            storage_diff.append(StorageEntry(
                key=storage_key,
                value=leaf.value
                )
            )

    return storage_diff


def generate_state_update(previous_state: CarriedState, current_state: CarriedState) -> BlockStateUpdate:
    """
    Returns roots, deployed contracts and storage diffs between 2 states
    """
    deployed_contracts: List[DeployedContract] = []
    declared_contracts: List[int] = []
    storage_diffs: Dict[int, List[StorageEntry]] = {}

    for class_hash in current_state.contract_definitions:
        if class_hash not in previous_state.contract_definitions:
            declared_contracts.append(
                int.from_bytes(class_hash, byteorder="big")
            )

    for contract_address in current_state.contract_states:
        if contract_address not in previous_state.contract_states:
            class_hash = int.from_bytes(
                current_state.contract_states[contract_address].state.contract_hash,
                "big"
            )
            deployed_contracts.append(
                DeployedContract(
                    address=contract_address,
                    class_hash=class_hash
                )
            )
        else:
            previous_storage_updates = previous_state.contract_states[contract_address].storage_updates
            storage_updates = current_state.contract_states[contract_address].storage_updates
            storage_diff = generate_storage_diff(previous_storage_updates, storage_updates)

            if len(storage_diff) > 0:
                storage_diffs[contract_address] = storage_diff

    new_root = current_state.shared_state.contract_states.root
    old_root = previous_state.shared_state.contract_states.root
    state_diff = StateDiff(
        deployed_contracts=deployed_contracts,
        declared_contracts=tuple(declared_contracts),
        storage_diffs=storage_diffs
    )

    return BlockStateUpdate(
        block_hash=None,
        new_root=new_root,
        old_root=old_root,
        state_diff=state_diff
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
