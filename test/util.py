"""
File containing functions that wrap Starknet CLI commands.
"""

import json
import os
import re
import subprocess
import time
import requests

from starkware.starknet.services.api.contract_class import ContractClass

from starknet_devnet.general_config import DEFAULT_GENERAL_CONFIG
from .settings import HOST, PORT, APP_URL

class ReturnCodeAssertionError(AssertionError):
    """Error to be raised when the return code of an executed process is not as expected."""

def run_devnet_in_background(*args, stderr=None, stdout=None):
    """
    Runs starknet-devnet in background.
    Sleep before devnet is responsive.
    Accepts extra args to pass to `starknet-devnet` command.
    Returns the process handle.
    """
    # If accounts argument is not passed, 1 is used as default
    # used to be 0, but this was causing problems
    if "--accounts" not in args:
        args = [*args, "--accounts", "1"]

    command = ["poetry", "run", "starknet-devnet", "--host", HOST, "--port", PORT, *args]
    # pylint: disable=consider-using-with
    proc = subprocess.Popen(command, close_fds=True, stderr=stderr, stdout=stdout)

    ensure_server_alive(f"{APP_URL}/is_alive", proc)
    return proc

def devnet_in_background(*devnet_args, **devnet_kwargs):
    """
    Decorator that runs devnet in background and later kills it.
    Prints devnet output in case of AssertionError.
    """
    def wrapper(func):
        def inner_wrapper(*args, **kwargs):
            proc = run_devnet_in_background(*devnet_args, **devnet_kwargs)
            try:
                func(*args, **kwargs)
            finally:
                terminate_and_wait(proc)
        return inner_wrapper
    return wrapper

def terminate_and_wait(proc: subprocess.Popen):
    """Terminates the process and waits."""
    proc.terminate()
    proc.wait()

def ensure_server_alive(url: str, proc: subprocess.Popen, check_period=0.5, max_wait=60):
    """
    Ensures that server at provided `url` is alive or that `proc` has terminated.
    Checks every `check_period` seconds.
    Fails after `max_wait` seconds - terminates the server's `proc`.
    """

    n_checks = int(max_wait // check_period)
    for _ in range(n_checks):
        if proc.poll() is not None:
            # if process has terminated, return
            return

        try:
            requests.get(url)
            return
        except requests.exceptions.ConnectionError:
            pass

        time.sleep(check_period)

    terminate_and_wait(proc)
    raise RuntimeError(f"max_wait time {max_wait} exceeded while checking {url}")

def assert_equal(actual, expected, explanation=None):
    """Assert that the two values are equal. Optionally provide explanation."""
    assert actual == expected, f"\nActual: {actual}\nExpected: {expected}\nAdditional_info: {explanation}"

def assert_hex_equal(actual, expected):
    """Assert that two hex strings are equal when converted to ints"""
    assert int(actual, 16) == int(expected, 16)

def extract(regex, stdout):
    """Extract from `stdout` what matches `regex`."""
    matched = re.search(regex, stdout)
    if matched:
        return matched.group(1)
    raise RuntimeError(f"Cannot extract from {stdout}")

def extract_class_hash(stdout):
    """Extract class hash from stdout."""
    return extract(r"Contract class hash: (\w*)", stdout)

def extract_tx_hash(stdout):
    """Extract tx_hash from stdout."""
    return extract(r"Transaction hash: (\w*)", stdout)

def extract_fee(stdout) -> int:
    """Extract fee from stdout."""
    return int(extract(r"(\d+)", stdout))

def extract_address(stdout):
    """Extract address from stdout."""
    return extract(r"Contract address: (\w*)", stdout)

def run_starknet(args, raise_on_nonzero=True, add_gateway_urls=True):
    """Wrapper around subprocess.run"""
    my_args = ["poetry", "run", "starknet", *args, "--no_wallet"]
    if add_gateway_urls:
        my_args.extend([
            "--gateway_url", APP_URL,
            "--feeder_gateway_url", APP_URL
        ])
    output = subprocess.run(my_args, encoding="utf-8", check=False, capture_output=True)
    if output.returncode != 0 and raise_on_nonzero:
        if output.stderr:
            raise ReturnCodeAssertionError(output.stderr)
        raise ReturnCodeAssertionError(output.stdout)
    return output

def declare(contract):
    """Wrapper around starknet declare"""
    args = ["declare", "--contract", contract]
    output = run_starknet(args)
    return {
        "tx_hash": extract_tx_hash(output.stdout),
        "class_hash": extract_class_hash(output.stdout)
    }

def deploy(contract, inputs=None, salt=None):
    """Wrapper around starknet deploy"""
    args = ["deploy", "--contract", contract]
    if inputs:
        args.extend(["--inputs", *inputs])
    if salt:
        args.extend(["--salt", salt])
    output = run_starknet(args)
    return {
        "tx_hash": extract_tx_hash(output.stdout),
        "address": extract_address(output.stdout)
    }

def assert_transaction(tx_hash, expected_status, expected_signature=None):
    """Wrapper around starknet get_transaction"""
    output = run_starknet(["get_transaction", "--hash", tx_hash])
    transaction = json.loads(output.stdout)
    assert_equal(transaction["status"], expected_status)
    if expected_signature:
        assert_equal(transaction["transaction"]["signature"], expected_signature)

    expected_keys = ["status", "transaction", "transaction_index"]
    if expected_status == "REJECTED":
        expected_keys.append("transaction_failure_reason")
    else:
        expected_keys.extend(["block_hash", "block_number"])

    assert_keys(transaction, expected_keys)

    tx_type = transaction["transaction"]["type"]

    if tx_type == "INVOKE_FUNCTION":
        invoke_transaction_keys = [
            "calldata", "contract_address", "entry_point_selector", "entry_point_type",
            "max_fee", "signature", "transaction_hash", "type"
        ]
        assert_keys(transaction["transaction"], invoke_transaction_keys)

    if tx_type == "DEPLOY":
        deploy_transaction_keys = [
            "class_hash", "constructor_calldata", "contract_address",
            "contract_address_salt", "transaction_hash", "type"
        ]
        assert_keys(transaction["transaction"], deploy_transaction_keys)

def assert_keys(dictionary, keys):
    """Asserts that the dict has the correct keys"""
    expected_set = set(keys)
    assert dictionary.keys() == expected_set, f"{dictionary.keys()} != {expected_set}"

def assert_transaction_not_received(tx_hash):
    """Assert correct tx response when there is no tx with `tx_hash`."""
    output = run_starknet(["get_transaction", "--hash", tx_hash])
    transaction = json.loads(output.stdout)
    assert_equal(transaction, {
        "status": "NOT_RECEIVED"
    })

def assert_transaction_receipt_not_received(tx_hash):
    """Assert correct tx receipt response when there is no tx with `tx_hash`."""
    receipt = get_transaction_receipt(tx_hash)
    assert_equal(receipt, {
        "events": [],
        "l2_to_l1_messages": [],
        "status": "NOT_RECEIVED",
        "transaction_hash": tx_hash
    })

# pylint: disable=too-many-arguments
def invoke(function, inputs, address, abi_path, signature=None, max_fee=None):
    """Wrapper around starknet invoke. Returns tx hash."""
    args = [
        "invoke",
        "--function", function,
        "--inputs", *inputs,
        "--address", address,
        "--abi", abi_path,
    ]
    if signature:
        args.extend(["--signature", *signature])

    if max_fee:
        args.extend(["--max_fee", max_fee])

    output = run_starknet(args)

    print("Invoke successful!")
    return extract_tx_hash(output.stdout)


def estimate_fee(function, inputs, address, abi_path, signature=None):
    """Wrapper around starknet estimate_fee. Returns fee in wei."""
    args = [
        "estimate_fee",
        "--function", function,
        "--inputs", *inputs,
        "--address", address,
        "--abi", abi_path,
    ]
    if signature:
        args.extend(["--signature", *signature])

    output = run_starknet(args)

    print("Estimate fee successful!")
    return extract_fee(output.stdout)


def call(function, address, abi_path, inputs=None, signature=None, max_fee=None):
    """Wrapper around starknet call"""
    args = [
        "call",
        "--function", function,
        "--address", address,
        "--abi", abi_path,
    ]
    if inputs:
        args.extend(["--inputs", *inputs])
    if signature:
        args.extend(["--signature", *signature])
    if max_fee:
        args.extend(["--max_fee", max_fee])

    output = run_starknet(args)

    print("Call successful!")
    return output.stdout.rstrip()

def load_contract_class(contract_path: str):
    """Loads the contract class from the contract path"""
    loaded_contract = load_json_from_path(contract_path)

    return ContractClass.load(loaded_contract)

def assert_tx_status(tx_hash, expected_tx_status):
    """Asserts the tx_status of the tx with tx_hash."""
    output = run_starknet(["tx_status", "--hash", tx_hash])
    response = json.loads(output.stdout)
    tx_status = response["tx_status"]
    assert_equal(tx_status, expected_tx_status, response)

    if tx_status == "REJECTED":
        assert "tx_failure_reason" in response, f"Key not found in {response}"

def assert_contract_code(address):
    """Asserts the content of the code of a contract at address."""
    output = run_starknet(["get_code", "--contract_address", address])
    code = json.loads(output.stdout)
    # just checking key equality
    assert_equal(sorted(code.keys()), ["abi", "bytecode"])

def assert_contract_class(actual_class: ContractClass, expected_class_path: str):
    """Asserts equality between `actual_class` and class at `expected_class_path`."""

    loaded_contract_class = load_contract_class(expected_class_path)
    assert_equal(actual_class, loaded_contract_class.remove_debug_info())

def assert_storage(address, key, expected_value):
    """Asserts the storage value stored at (address, key)."""
    output = run_starknet([
        "get_storage_at",
        "--contract_address", address,
        "--key", key
    ])
    assert_equal(output.stdout.rstrip(), expected_value)

def load_json_from_path(path):
    """Loads a json file from `path`."""
    with open(path, encoding="utf-8") as expected_file:
        return json.load(expected_file)

def get_transaction_receipt(tx_hash):
    """Fetches the transaction receipt of transaction with tx_hash"""
    output = run_starknet(["get_transaction_receipt", "--hash", tx_hash])
    return json.loads(output.stdout)

def get_full_contract(contract_address: str) -> ContractClass:
    """Gets contract class by contract address"""
    output = run_starknet(["get_full_contract", "--contract_address", contract_address])
    return ContractClass.loads(output.stdout)

def get_class_hash_at(contract_address: str) -> str:
    """Gets class hash at given contract address"""
    output = run_starknet(["get_class_hash_at", "--contract_address", contract_address])
    return json.loads(output.stdout)

def get_class_by_hash(class_hash: str) -> str:
    """Gets contract class by contract hash"""
    output = run_starknet(["get_class_by_hash", "--class_hash", class_hash])
    return ContractClass.loads(output.stdout)

def assert_receipt(tx_hash, expected_path):
    """Asserts the content of the receipt of tx with tx_hash."""
    receipt = get_transaction_receipt(tx_hash)
    expected_receipt = load_json_from_path(expected_path)

    assert_equal(receipt["transaction_hash"], tx_hash)

    for ignorable_key in ["block_hash", "transaction_hash"]:
        receipt.pop(ignorable_key)
        expected_receipt.pop(ignorable_key)
    assert_equal(receipt, expected_receipt)

def assert_events(tx_hash, expected_path):
    """Asserts the content of the events element of the receipt of tx with tx_hash."""
    receipt = get_transaction_receipt(tx_hash)
    expected_receipt = load_json_from_path(expected_path)
    assert_equal(receipt["events"], expected_receipt["events"])

def get_block(block_number=None, parse=False):
    """Get the block with block_number. If no number provided, return the last."""
    args = ["get_block"]
    if block_number:
        args.extend(["--number", str(block_number)])
    if parse:
        output = run_starknet(args, raise_on_nonzero=True)
        return json.loads(output.stdout)

    return run_starknet(args, raise_on_nonzero=False)

def assert_negative_block_input():
    """Test behavior if get_block provided with negative input."""
    try:
        get_block(-1, parse=True)
        raise RuntimeError("Should have failed on negative block number")
    except ReturnCodeAssertionError:
        print("Correctly rejecting negative block number")

def assert_block(latest_block_number, latest_tx_hash):
    """Asserts the content of the block with block_number."""
    too_big = 1000
    error_message = get_block(block_number=too_big, parse=False).stderr
    total_blocks_str = re.search("There are currently (.*) blocks.", error_message).group(1)
    total_blocks = int(total_blocks_str)
    extracted_last_block_number = total_blocks - 1
    assert_equal(extracted_last_block_number, latest_block_number)

    latest_block = get_block(parse=True)
    specific_block = get_block(block_number=extracted_last_block_number, parse=True)
    assert_equal(latest_block, specific_block)

    assert_equal(latest_block["block_number"], latest_block_number)
    assert_equal(latest_block["status"], "ACCEPTED_ON_L2")

    latest_block_transactions = latest_block["transactions"]
    assert_equal(len(latest_block_transactions), 1)
    latest_transaction = latest_block_transactions[0]
    assert_equal(latest_transaction["transaction_hash"], latest_tx_hash)

    assert_equal(latest_block["sequencer_address"], hex(DEFAULT_GENERAL_CONFIG.sequencer_address))
    assert_equal(latest_block["gas_price"], hex(DEFAULT_GENERAL_CONFIG.min_gas_price))

def assert_block_hash(latest_block_number, expected_block_hash):
    """Asserts the content of the block with block_number."""

    block = get_block(block_number=latest_block_number, parse=True)
    assert_equal(block["block_hash"], expected_block_hash)
    assert_equal(block["status"], "ACCEPTED_ON_L2")

def assert_salty_deploy(contract_path, inputs, salt, expected_status, expected_address, expected_tx_hash):
    """Deploy with salt and assert."""

    deploy_info = deploy(contract_path, inputs, salt=salt)
    assert_tx_status(deploy_info["tx_hash"], expected_status)
    assert_equal(deploy_info["address"], expected_address)
    assert_equal(deploy_info["tx_hash"], expected_tx_hash)

def assert_failing_deploy(contract_path):
    """Run deployment for a contract that's expected to be rejected."""
    deploy_info = deploy(contract_path)
    assert_tx_status(deploy_info["tx_hash"], "REJECTED")
    assert_transaction(deploy_info["tx_hash"], "REJECTED")

def load_file_content(file_name: str):
    """Load content of file located in the same directory as this test file."""
    full_file_path = os.path.join(os.path.dirname(__file__), file_name)
    with open(full_file_path, encoding="utf-8") as deploy_file:
        return deploy_file.read()

def create_empty_block():
    """Creates an empty block and returns it."""
    resp = requests.post(f"{APP_URL}/create_block")
    assert resp.status_code == 200
    return resp.json()
