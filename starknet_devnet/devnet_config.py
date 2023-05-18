"""Module for configuration specified by user"""

import argparse
import asyncio
import json
import os
import subprocess
import sys
from enum import Enum, auto
from typing import List

from aiohttp.client_exceptions import ClientConnectorError, InvalidURL
from marshmallow.exceptions import ValidationError
from services.external_api.client import BadRequest, RetryConfig
from starkware.starknet.core.os.contract_class.deprecated_class_hash import (
    compute_deprecated_class_hash,
)
from starkware.starknet.definitions.general_config import StarknetChainId
from starkware.starknet.services.api.contract_class.contract_class import (
    DeprecatedCompiledClass,
)
from starkware.starknet.services.api.feeder_gateway.feeder_gateway_client import (
    FeederGatewayClient,
)

from starknet_devnet.util import suppress_feeder_gateway_client_logger

from . import __version__
from .constants import (
    DEFAULT_ACCOUNTS,
    DEFAULT_GAS_PRICE,
    DEFAULT_HOST,
    DEFAULT_INITIAL_BALANCE,
    DEFAULT_PORT,
    DEFAULT_TIMEOUT,
)
from .contract_class_wrapper import (
    DEFAULT_ACCOUNT_HASH,
    DEFAULT_ACCOUNT_PATH,
    CompiledClassWrapper,
)

NETWORK_TO_URL = {
    "alpha-goerli": "https://alpha4.starknet.io",
    "alpha-goerli2": "https://alpha4-2.starknet.io",
    "alpha-mainnet": "https://alpha-mainnet.starknet.io",
}
NETWORK_NAMES = ", ".join(NETWORK_TO_URL.keys())
CHAIN_IDS = ", ".join([member.name for member in StarknetChainId])
DEFAULT_CHAIN_ID = StarknetChainId.TESTNET


def _fork_network(network_id: str):
    """
    Return the URL corresponding to the provided name.
    If it's not one of predefined names, assumes it is already a URL.
    """
    return NETWORK_TO_URL.get(network_id, network_id)


def _fork_block(specifier: str):
    """Parse block specifier; allows int and 'latest'"""
    if specifier == "latest":
        return specifier

    try:
        parsed = int(specifier)
        assert parsed > 0
    except (AssertionError, ValueError):
        sys.exit(
            f"The value of --fork-block must be a non-negative integer or 'latest', got: {specifier}"
        )

    return parsed


def _chain_id(chain_id: str):
    """Parse chain id.'"""
    try:
        chain_id = StarknetChainId[chain_id]
    except KeyError:
        sys.exit(
            f"Error: The value of --chain-id must be in {{{CHAIN_IDS}}}, got: {chain_id}"
        )

    return chain_id


class DumpOn(Enum):
    """Enumerate possible dumping frequencies."""

    EXIT = auto()
    TRANSACTION = auto()


DUMP_ON_OPTIONS = [e.name.lower() for e in DumpOn]
DUMP_ON_OPTIONS_STRINGIFIED = ", ".join(DUMP_ON_OPTIONS)


def _parse_dump_on(option: str):
    """Parse dumping frequency option."""
    if option in DUMP_ON_OPTIONS:
        return DumpOn[option.upper()]
    sys.exit(
        f"Error: Invalid --dump-on option: {option}. Valid options: {DUMP_ON_OPTIONS_STRINGIFIED}"
    )


EXPECTED_ACCOUNT_METHODS = ["__execute__", "__validate__", "__validate_declare__"]


def _parse_account_class(class_path: str) -> CompiledClassWrapper:
    """Parse account class"""
    class_path = os.path.abspath(class_path)

    if not os.path.isfile(class_path):
        sys.exit(f"Error: {class_path} is not a valid file")

    with open(class_path, mode="r", encoding="utf-8") as dict_file:
        try:
            loaded_dict = json.load(dict_file)
        except json.JSONDecodeError:
            sys.exit(f"Error: {class_path} is not a valid JSON file")

    try:
        contract_class = DeprecatedCompiledClass.load(loaded_dict)
    except ValidationError:
        sys.exit(f"Error: {class_path} is not a valid contract class artifact")

    if class_path == DEFAULT_ACCOUNT_PATH:
        class_hash = DEFAULT_ACCOUNT_HASH
    else:
        contract_methods = [entry["name"] for entry in contract_class.abi]
        missing_methods = [
            m for m in EXPECTED_ACCOUNT_METHODS if m not in contract_methods
        ]
        if missing_methods:
            sys.exit(
                f"Error: {class_path} is missing account methods: {', '.join(missing_methods)}"
            )
        class_hash = compute_deprecated_class_hash(contract_class)

    return CompiledClassWrapper(contract_class, class_hash)


def _get_feeder_gateway_client(url: str, block_id: str, n_retries: int = 1):
    """Construct a feeder gateway client at url and block"""

    feeder_gateway_client = FeederGatewayClient(
        url=url,
        retry_config=RetryConfig(n_retries=n_retries),
    )

    try:
        with suppress_feeder_gateway_client_logger:
            block = asyncio.run(feeder_gateway_client.get_block(block_number=block_id))
            block_number = block.block_number
    except InvalidURL:
        sys.exit(
            f"Error: Invalid fork-network (must be a URL or one of {{{NETWORK_NAMES}}}). Received: {url}"
        )
    except BadRequest as bad_request:
        if bad_request.status_code == 404:
            msg = f"Error: {url} is not a valid Starknet sequencer"
        else:
            msg = f"Error: {bad_request}"

        sys.exit(msg)
    except ClientConnectorError as error:
        sys.exit(f"Error: {error}")

    return feeder_gateway_client, block_number


class NonNegativeAction(argparse.Action):
    """
    Action for parsing the non negative int argument.
    """

    def __call__(self, parser, namespace, values, option_string=None):
        error_msg = f"{option_string} must be a positive integer; got: {values}."
        try:
            value = int(values)
        except ValueError:
            parser.error(error_msg)

        if value < 0:
            parser.error(error_msg)

        setattr(namespace, self.dest, value)


class PositiveAction(argparse.Action):
    """
    Action for parsing positive int argument;
    """

    def __call__(self, parser, namespace, values, option_string=None):
        error_msg = (
            f"argument {option_string} must be a positive integer; got: {values}."
        )
        try:
            value = int(values)
        except ValueError:
            parser.error(error_msg)

        if value <= 0:
            parser.error(error_msg)

        setattr(namespace, self.dest, value)


def _assert_valid_compiler(command: List[str]):
    """Assert user machine can compile with cairo 1"""
    check = subprocess.run(
        command,
        check=False,
        capture_output=True,
    )

    if check.returncode:
        stderr_content = check.stderr.decode("utf-8")
        sys.exit(f"Cairo compiler error: {stderr_content}")

    version_used = check.stdout.decode("utf-8")
    print(f"Using cairo compiler: {version_used}")


def _parse_cairo_compiler_manifest(manifest_path: str):
    command = [
        "cargo",
        "run",
        "--bin",
        "starknet-sierra-compile",
        "--manifest-path",
        manifest_path,
        "--",
        "--version",
    ]
    _assert_valid_compiler(command)

    return manifest_path


def _parse_sierra_compiler_path(compiler_path: str):
    if not (os.path.isfile(compiler_path) and os.access(compiler_path, os.X_OK)):
        sys.exit("Error: The argument of --sierra-compiler-path must be an executable")

    _assert_valid_compiler([compiler_path, "--version"])
    return compiler_path


def parse_args(raw_args: List[str]):
    """
    Parses CLI arguments.
    """
    parser = argparse.ArgumentParser(
        description="Run a local instance of Starknet Devnet"
    )
    parser.add_argument(
        "-v",
        "--version",
        help="Print the version",
        action="version",
        version=__version__,
    )
    parser.add_argument(
        "--host",
        help=f"Specify the address to listen at; defaults to {DEFAULT_HOST} "
        "(use the address the program outputs on start)",
        default=DEFAULT_HOST,
    )
    parser.add_argument(
        "--port",
        "-p",
        type=int,
        help=f"Specify the port to listen at; defaults to {DEFAULT_PORT}",
        default=DEFAULT_PORT,
    )
    parser.add_argument(
        "--load-path", help="Specify the path from which the state is loaded on startup"
    )
    parser.add_argument("--dump-path", help="Specify the path to dump to")
    parser.add_argument(
        "--dump-on",
        help=f"Specify when to dump; can dump on: {DUMP_ON_OPTIONS_STRINGIFIED}",
        type=_parse_dump_on,
    )
    parser.add_argument(
        "--lite-mode",
        action="store_true",
        help="Introduces speed-up by skipping block hash calculation"
        " - applies sequential numbering instead (0x0, 0x1, 0x2, ...).",
    )
    parser.add_argument(
        "--blocks-on-demand",
        action="store_true",
        help="Block generation on demand via an endpoint.",
    )
    parser.add_argument(
        "--accounts",
        action=NonNegativeAction,
        help=f"Specify the number of accounts to be predeployed; defaults to {DEFAULT_ACCOUNTS}",
        default=DEFAULT_ACCOUNTS,
    )
    parser.add_argument(
        "--initial-balance",
        "-e",
        action=NonNegativeAction,
        help="Specify the initial balance of accounts to be predeployed; "
        f"defaults to {DEFAULT_INITIAL_BALANCE:g}",
        default=DEFAULT_INITIAL_BALANCE,
    )
    parser.add_argument(
        "--seed",
        type=int,
        help="Specify the seed for randomness of accounts to be predeployed",
    )
    parser.add_argument(
        "--hide-predeployed-accounts",
        action="store_true",
        help="Prevents from printing the predeployed accounts details",
    )
    parser.add_argument(
        "--start-time",
        action=NonNegativeAction,
        help="Specify the start time of the genesis block in Unix time seconds",
    )
    parser.add_argument(
        "--gas-price",
        "-g",
        action=NonNegativeAction,
        default=DEFAULT_GAS_PRICE,
        help=f"Specify the gas price in wei per gas unit; defaults to {DEFAULT_GAS_PRICE:g}",
    )
    parser.add_argument(
        "--allow-max-fee-zero",
        action="store_true",
        help="Allow transactions with max fee equal to zero",
    )
    parser.add_argument(
        "--timeout",
        "-t",
        action=NonNegativeAction,
        default=DEFAULT_TIMEOUT,
        help=f"Specify the server timeout in seconds; defaults to {DEFAULT_TIMEOUT}",
    )
    parser.add_argument(
        "--account-class",
        help="Specify the account implementation to be used for predeploying; "
        "should be a path to the compiled JSON artifact; "
        "defaults to OpenZeppelin v1",
        type=_parse_account_class,
        default=DEFAULT_ACCOUNT_PATH,
    )
    parser.add_argument(
        "--fork-network",
        type=_fork_network,
        help="Specify the network to fork: can be a URL (e.g. https://alpha-mainnet.starknet.io) "
        f"or network name (valid names: {', '.join(NETWORK_TO_URL.keys())})",
    )
    parser.add_argument(
        "--fork-block",
        type=_fork_block,
        help="Specify the block number where the --fork-network is forked; defaults to latest",
    )
    parser.add_argument(
        "--fork-retries",
        type=int,
        default=1,
        action=PositiveAction,
        help="Specify the number of retries of failed HTTP requests sent to the network before giving up; defaults to 1",
    )
    parser.add_argument(
        "--chain-id",
        type=_chain_id,
        default=DEFAULT_CHAIN_ID,
        help=f"Specify the chain id as one of: {{{CHAIN_IDS}}}; defaults to {DEFAULT_CHAIN_ID.name} ({hex(DEFAULT_CHAIN_ID.value)})",
    )
    parser.add_argument(
        "--disable-rpc-request-validation",
        action="store_true",
        help="Disable requests schema validation for RPC endpoints",
    )
    parser.add_argument(
        "--disable-rpc-response-validation",
        action="store_true",
        help="Disable RPC schema validation for devnet responses",
    )
    parser.add_argument(
        "--cairo-compiler-manifest",
        type=_parse_cairo_compiler_manifest,
        help="Specify the path to the manifest (Cargo.toml) of the Cairo 1.0 compiler to be used for contract recompilation; "
        "if omitted, the default x86-compatible compiler (from cairo-lang package) is used",
    )
    parser.add_argument(
        "--sierra-compiler-path",
        type=_parse_sierra_compiler_path,
        help="Specify the path to the binary executable of starknet-sierra-compile",
    )

    parsed_args = parser.parse_args(raw_args)
    if parsed_args.dump_on and not parsed_args.dump_path:
        sys.exit("Error: --dump-path required if --dump-on present")

    if parsed_args.fork_block and not parsed_args.fork_network:
        sys.exit("Error: --fork-network required if --fork-block present")

    if parsed_args.fork_network:
        parsed_args.fork_block = parsed_args.fork_block or "latest"
        parsed_args.fork_network, parsed_args.fork_block = _get_feeder_gateway_client(
            parsed_args.fork_network, parsed_args.fork_block, parsed_args.fork_retries
        )

    if parsed_args.cairo_compiler_manifest and parsed_args.sierra_compiler_path:
        sys.exit(
            "Error: Only one of {--cairo-compiler-manifest,--sierra-compiler-path} can be provided"
        )

    return parsed_args


# pylint: disable=too-few-public-methods
# pylint: disable=too-many-instance-attributes
class DevnetConfig:
    """Class holding configuration specified by user"""

    def __init__(self, args: argparse.Namespace = None):
        # these args are used in tests; in production, this is overwritten in `main`
        self.args = args or parse_args(["--accounts", "0"])
        self.accounts = self.args.accounts
        self.initial_balance = self.args.initial_balance
        self.seed = self.args.seed
        self.start_time = self.args.start_time
        self.gas_price = self.args.gas_price
        self.allow_max_fee_zero = self.args.allow_max_fee_zero
        self.lite_mode = self.args.lite_mode
        self.blocks_on_demand = self.args.blocks_on_demand
        self.account_class = self.args.account_class
        self.hide_predeployed_accounts = self.args.hide_predeployed_accounts
        self.fork_network = self.args.fork_network
        self.fork_block = self.args.fork_block
        self.chain_id = self.args.chain_id
        self.validate_rpc_requests = not self.args.disable_rpc_request_validation
        self.validate_rpc_responses = not self.args.disable_rpc_response_validation
        self.cairo_compiler_manifest = self.args.cairo_compiler_manifest
        self.sierra_compiler_path = self.args.sierra_compiler_path
