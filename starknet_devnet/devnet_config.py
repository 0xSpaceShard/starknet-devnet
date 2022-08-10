"""Module for configuration specified by user"""

import argparse
from enum import Enum, auto
import sys

from . import __version__
from .constants import (
    DEFAULT_ACCOUNTS,
    DEFAULT_GAS_PRICE,
    DEFAULT_HOST,
    DEFAULT_INITIAL_BALANCE,
    DEFAULT_PORT
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
        action="store_true",
        help="Applies all lite-mode-* optimizations by disabling some features."
    )
    parser.add_argument(
        "--lite-mode-block-hash",
        action="store_true",
        help="Disables block hash calculation"
    )
    parser.add_argument(
        "--lite-mode-deploy-hash",
        action="store_true",
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

# pylint: disable=too-few-public-methods
# pylint: disable=too-many-instance-attributes
class DevnetConfig:
    """Class holding configuration specified by user"""

    def __init__(self, args: argparse.Namespace=None):
        self.args = args
        self.accounts = args.accounts
        self.initial_balance = args.initial_balance
        self.seed = args.seed
        self.start_time = args.start_time
        self.gas_price = args.gas_price

        if args.lite_mode:
            self.lite_mode_block_hash = True
            self.lite_mode_deploy_hash = True
        else:
            self.lite_mode_block_hash = args.lite_mode_block_hash
            self.lite_mode_deploy_hash = args.lite_mode_deploy_hash

devnet_config = DevnetConfig(parse_args())
