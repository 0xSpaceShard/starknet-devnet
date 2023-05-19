"""
File containing functions that wrap Starknet CLI commands.
"""

import functools
import json
import os
import re
import subprocess
import time
from typing import IO, List, Optional

import pytest
import requests
from starkware.starknet.definitions.error_codes import StarknetErrorCode
from starkware.starknet.definitions.general_config import StarknetChainId
from starkware.starknet.definitions.transaction_type import TransactionType
from starkware.starknet.services.api.contract_class.contract_class import (
    CompiledClass,
    CompiledClassBase,
    DeprecatedCompiledClass,
)
from starkware.starknet.services.api.feeder_gateway.response_objects import (
    TransactionInfo,
)

from starknet_devnet.general_config import DEFAULT_GENERAL_CONFIG

from .settings import APP_URL, HOST, PORT


class ReturnCodeAssertionError(AssertionError):
    """Error to be raised when the return code of an executed process is not as expected."""


def run_devnet_in_background(*args, stderr=None, stdout=None, env=None):
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

    port = args[args.index("--port") + 1] if "--port" in args else PORT

    command = [
        "poetry",
        "run",
        "starknet-devnet",
        "--host",
        HOST,
        "--port",
        port,
        *args,
    ]
    # pylint: disable=consider-using-with
    proc = subprocess.Popen(
        command, close_fds=True, stderr=stderr, stdout=stdout, env=env
    )

    healthcheck_url = f"http://{HOST}:{port}/is_alive"
    ensure_server_alive(healthcheck_url, proc)
    return proc


def devnet_in_background(*devnet_args, **devnet_kwargs):
    """
    Decorator that runs devnet in background and later kills it.
    Prints devnet output in case of AssertionError.
    """

    def wrapper(func):
        @functools.wraps(func)
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


def ensure_server_alive(
    url: str, proc: subprocess.Popen, check_period=0.5, max_wait=60
):
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
    assert (
        actual == expected
    ), f"\nActual: {actual}\nExpected: {expected}\nAdditional_info: {explanation}"


def assert_hex_equal(actual, expected):
    """
    Assert that two hex strings are equal when converted to ints.
    Converting back to hex to have hex strings in error message in case of failed assertion.
    """
    assert hex(int(actual, 16)) == hex(int(expected, 16))


def assert_get_events_response(
    resp: dict, expected_block_length: int, expected_token: Optional[str] = None
):
    """
    If expected_token is None, check that it is not present, if it's not None, check if the returned value matches
    the one provided.
    """
    assert len(resp["result"]["events"]) == expected_block_length
    if expected_token is not None:
        assert resp["result"]["continuation_token"] == expected_token
    else:
        assert "continuation_token" not in resp["result"]


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
    return int(extract(r"The estimated fee is: (\d+) WEI", stdout))


def extract_address(stdout):
    """Extract address from stdout."""
    return extract(r"Contract address: (\w*)", stdout)


def run_starknet(args, raise_on_nonzero=True, gateway_url=APP_URL):
    """Wrapper around subprocess.run"""
    my_args = ["poetry", "run", "starknet", *args, "--no_wallet"]
    # there is no case when gateway should not be equal to feeder gateway
    my_args.extend(["--gateway_url", gateway_url, "--feeder_gateway_url", gateway_url])

    output = subprocess.run(my_args, encoding="utf-8", check=False, capture_output=True)
    if output.returncode != 0 and raise_on_nonzero:
        if output.stderr:
            raise ReturnCodeAssertionError(output.stderr)
        raise ReturnCodeAssertionError(output.stdout)
    return output


def send_tx(transaction: dict, tx_type: TransactionType, gateway_url=APP_URL) -> dict:
    """
    Send transaction.
    Returns tx hash
    """
    resp = requests.post(
        url=f"{gateway_url}/gateway/add_transaction",
        json={**transaction, "type": tx_type.name},
    )
    assert resp.status_code == 200, resp.json()
    return resp.json()


def estimate_message_fee(
    from_address: str, function: str, inputs: List[str], to_address: str, abi_path: str
):
    """Wrapper around starknet estimate_message_fee"""
    output = run_starknet(
        [
            "estimate_message_fee",
            "--from_address",
            from_address,
            "--function",
            function,
            "--inputs",
            *inputs,
            "--address",
            to_address,
            "--abi",
            abi_path,
        ]
    )

    return extract_fee(output.stdout)


def assert_transaction(
    tx_hash, expected_status, expected_signature=None, feeder_gateway_url=APP_URL
):
    """Wrapper around starknet get_transaction"""
    output = run_starknet(
        ["get_transaction", "--hash", tx_hash], gateway_url=feeder_gateway_url
    )

    transaction: TransactionInfo = TransactionInfo.loads(output.stdout)
    assert_equal(transaction.status.name, expected_status, transaction)

    if expected_signature:
        assert_equal(transaction.transaction.signature, expected_signature)


def assert_transaction_not_received(tx_hash: str, feeder_gateway_url=APP_URL):
    """Assert correct tx response when there is no tx with `tx_hash`."""
    output = run_starknet(
        ["get_transaction", "--hash", tx_hash], gateway_url=feeder_gateway_url
    )
    transaction = json.loads(output.stdout)
    assert_equal(transaction, {"status": "NOT_RECEIVED"})


def assert_transaction_receipt_not_received(tx_hash: str, feeder_gateway_url=APP_URL):
    """Assert correct tx receipt response when there is no tx with `tx_hash`."""
    receipt = get_transaction_receipt(tx_hash, feeder_gateway_url=feeder_gateway_url)
    assert_equal(
        receipt,
        {
            "events": [],
            "l2_to_l1_messages": [],
            "status": "NOT_RECEIVED",
            "transaction_hash": "0x0",
        },
    )


def _add_block_specifier(args: List[str], block_number=None, block_hash=None):
    if block_number is not None:
        args.extend(["--block_number", block_number])

    if block_hash is not None:
        args.extend(["--block_hash", block_hash])


# pylint: disable=too-many-arguments
def estimate_fee(
    function,
    inputs,
    address,
    abi_path,
    signature=None,
    nonce=None,
    block_number=None,
    block_hash=None,
    chain_id=StarknetChainId.TESTNET,
    feeder_gateway_url=APP_URL,
):
    """Wrapper around starknet estimate_fee. Returns fee in wei."""
    args = [
        "invoke",
        "--estimate_fee",
        "--function",
        function,
        "--inputs",
        *inputs,
        "--address",
        address,
        "--abi",
        abi_path,
        "--chain_id",
        hex(chain_id.value),
    ]

    if signature:
        args.extend(["--signature", *signature])

    if nonce is not None:
        args.extend(["--nonce", str(nonce)])

    _add_block_specifier(args, block_number=block_number, block_hash=block_hash)

    output = run_starknet(args, gateway_url=feeder_gateway_url)

    print("Estimate fee successful!")
    return extract_fee(output.stdout)


def call(
    function: str,
    address: str,
    abi_path: Optional[str] = None,
    inputs=None,
    block_number=None,  # Starknet CLI defaults to pending - we shouldn't rely on that
    block_hash=None,
    feeder_gateway_url=APP_URL,
):
    """Wrapper around starknet call"""
    args = [
        "call",
        "--function",
        function,
        "--address",
        address,
    ]

    if abi_path:
        args.extend(["--abi", abi_path])

    if inputs:
        args.extend(["--inputs", *inputs])

    _add_block_specifier(args, block_number=block_number, block_hash=block_hash)

    output = run_starknet(args, gateway_url=feeder_gateway_url)

    print("Call successful!")
    return output.stdout.rstrip()


def load_contract_class(contract_path: str):
    """Loads the contract class from the contract path"""
    loaded_contract = load_json_from_path(contract_path)

    return DeprecatedCompiledClass.load(loaded_contract)


def assert_tx_status(tx_hash, expected_tx_status: str, feeder_gateway_url=APP_URL):
    """Asserts the tx_status of the tx with tx_hash."""
    output = run_starknet(
        ["tx_status", "--hash", tx_hash], gateway_url=feeder_gateway_url
    )
    response = json.loads(output.stdout)
    assert "tx_status" in response
    tx_status = response["tx_status"]
    assert_equal(tx_status, expected_tx_status, response)

    if tx_status == "REJECTED":
        assert "tx_failure_reason" in response, f"Key not found in {response}"


def assert_contract_code_present(address: str, feeder_gateway_url=APP_URL):
    """Asserts the content of the code of a contract at `address`."""
    output = run_starknet(
        ["get_code", "--contract_address", address], gateway_url=feeder_gateway_url
    )
    code = json.loads(output.stdout)

    assert code["abi"]  # assert non-empty
    assert code["bytecode"]  # assert non-empty

    # assert no other keys
    assert_equal(sorted(code.keys()), ["abi", "bytecode"])


def assert_contract_code_not_present(address: str, feeder_gateway_url=APP_URL):
    """Assert abi and bytecode empty"""
    resp = requests.get(
        f"{feeder_gateway_url}/feeder_gateway/get_code?contractAddress={address}"
    )

    code = resp.json()
    assert code["abi"] == {}
    assert code["bytecode"] == []

    assert resp.status_code == 200


def assert_contract_class(actual_class: CompiledClassBase, expected_class_path: str):
    """Asserts equality between `actual_class` and class at `expected_class_path`."""

    loaded_contract_class = load_contract_class(expected_class_path)
    assert_equal(actual_class, loaded_contract_class.remove_debug_info())


def assert_storage(
    address: str,
    key: str,
    expected_value: str,
    block_number=None,
    block_hash=None,
    feeder_gateway_url=APP_URL,
):
    """Asserts the storage value stored at (address, key)."""
    args = ["get_storage_at", "--contract_address", address, "--key", key]
    _add_block_specifier(args, block_number=block_number, block_hash=block_hash)

    output = run_starknet(
        args,
        gateway_url=feeder_gateway_url,
    )
    assert_equal(output.stdout.rstrip(), expected_value)


def load_json_from_path(path):
    """Loads a json file from `path`."""
    with open(path, encoding="utf-8") as expected_file:
        return json.load(expected_file)


def get_transaction_receipt(tx_hash: str, feeder_gateway_url=APP_URL):
    """Fetches the transaction receipt of transaction with tx_hash"""
    output = run_starknet(
        ["get_transaction_receipt", "--hash", tx_hash], gateway_url=feeder_gateway_url
    )
    return json.loads(output.stdout)


def get_full_contract(
    contract_address: str, feeder_gateway_url=APP_URL
) -> CompiledClassBase:
    """Gets contract class by contract address"""
    contract = get_full_contract_raw(contract_address, feeder_gateway_url)
    return DeprecatedCompiledClass.load(contract.json())


def get_full_contract_raw(contract_address: str, feeder_gateway_url=APP_URL):
    """Gets contract raw data as dump"""
    return requests.get(
        f"{feeder_gateway_url}/feeder_gateway/get_full_contract",
        {"contractAddress": contract_address},
    )


def assert_full_contract_not_present(address: str, feeder_gateway_url=APP_URL):
    """Assert that get_full_contract fails due to uninitialized contract"""
    resp = requests.get(
        f"{feeder_gateway_url}/feeder_gateway/get_full_contract",
        {"contractAddress": address},
    )

    assert resp.json()["code"] == str(StarknetErrorCode.UNINITIALIZED_CONTRACT)
    assert resp.status_code == 500


def assert_full_contract(address: str, expected_path: str, feeder_gateway_url=APP_URL):
    """Assert that the provided address has contract from `expected_path` deployed at it."""
    class_by_address = get_full_contract(
        contract_address=address, feeder_gateway_url=feeder_gateway_url
    )
    assert_contract_class(class_by_address, expected_class_path=expected_path)


def get_class_hash_at(contract_address: str) -> str:
    """Gets class hash at given contract address"""
    output = run_starknet(["get_class_hash_at", "--contract_address", contract_address])
    return output.stdout


def assert_address_has_no_class_hash(contract_address: str, feeder_gateway_url=APP_URL):
    """There should be no class hash at `contract_address`."""
    resp = requests.get(
        f"{feeder_gateway_url}/feeder_gateway/get_class_hash_at",
        {"contractAddress": contract_address},
    )
    assert resp.json()["code"] == str(StarknetErrorCode.UNINITIALIZED_CONTRACT)
    assert resp.status_code == 500


def assert_class_hash_at_address(
    contract_address: str, expected_class_hash: str, feeder_gateway_url=APP_URL
):
    """The class hash at `contract_address` should be `expected_class_hash`."""
    resp = requests.get(
        f"{feeder_gateway_url}/feeder_gateway/get_class_hash_at",
        {"contractAddress": contract_address},
    )
    received_class_hash = int(json.loads(resp.text), 16)
    assert received_class_hash == int(expected_class_hash, 16)
    assert resp.status_code == 200


def get_class_by_hash(class_hash: str, feeder_gateway_url=APP_URL):
    """Gets contract class by contract hash"""
    return requests.get(
        f"{feeder_gateway_url}/feeder_gateway/get_class_by_hash",
        {"classHash": class_hash},
    )


def get_compiled_class_by_class_hash(class_hash: str, feeder_gateway_url=APP_URL):
    """Gets compiled class by sierra hash"""
    return requests.get(
        f"{feeder_gateway_url}/feeder_gateway/get_compiled_class_by_class_hash",
        {"classHash": class_hash},
    )


def assert_class_by_hash(
    class_hash: str, expected_path: str, feeder_gateway_url=APP_URL
):
    """Assert the class at `class_hash` matches what is at `expected_path`."""
    resp = get_class_by_hash(class_hash, feeder_gateway_url=feeder_gateway_url)
    assert resp.status_code == 200, resp.text
    class_by_hash = DeprecatedCompiledClass.loads(resp.text).remove_debug_info()
    assert_contract_class(class_by_hash, expected_class_path=expected_path)


def assert_compiled_class_by_hash(
    class_hash: str,
    expected_path: str,
    feeder_gateway_url=APP_URL,
):
    """Assert the compiled class at `class_hash` matches what is at `expected_path`."""
    resp = get_compiled_class_by_class_hash(
        class_hash, feeder_gateway_url=feeder_gateway_url
    )
    assert resp.status_code == 200, resp.text
    retrieved_class = CompiledClass.loads(resp.text)
    with open(expected_path, encoding="utf-8") as expected_file:
        class_from_disk = CompiledClass.loads(expected_file.read())

    assert retrieved_class == class_from_disk


def assert_class_by_hash_not_present(class_hash: str, feeder_gateway_url=APP_URL):
    """Assert the server holds no class at provided `class_hash`."""
    resp = get_class_by_hash(class_hash, feeder_gateway_url=feeder_gateway_url)
    assert_undeclared_class(resp)


def assert_compiled_class_by_hash_not_present(
    class_hash: str, feeder_gateway_url=APP_URL
):
    """Assert the server holds no compiled class corresponding to the provided `class_hash`."""
    resp = get_compiled_class_by_class_hash(
        class_hash, feeder_gateway_url=feeder_gateway_url
    )
    assert_undeclared_class(resp)


def assert_receipt(tx_hash: str, expected_status: str):
    """Asserts the content of the receipt of tx with tx_hash."""
    receipt = get_transaction_receipt(tx_hash)

    assert_equal(receipt["transaction_hash"], tx_hash)
    assert_equal(receipt["status"], expected_status)


def assert_receipt_present(
    tx_hash: str, expected_status: str, feeder_gateway_url=APP_URL
):
    """Asserts the content of the receipt of tx with tx_hash is non-empty"""
    receipt = get_transaction_receipt(tx_hash, feeder_gateway_url=feeder_gateway_url)
    assert receipt["transaction_hash"] == tx_hash
    assert receipt["status"] == expected_status


def assert_events(tx_hash, expected_path):
    """Asserts the content of the events element of the receipt of tx with tx_hash."""
    receipt = get_transaction_receipt(tx_hash)
    expected_receipt = load_json_from_path(expected_path)
    assert_equal(receipt["events"], expected_receipt["events"])


def get_block(
    block_number=None, block_hash=None, parse=False, feeder_gateway_url=APP_URL
):
    """Get the block with block_number. If no number provided, return the last."""
    args = ["get_block"]
    if block_number:
        args.extend(["--number", str(block_number)])
    if block_hash:
        args.extend(["--hash", str(block_hash)])

    if parse:
        output = run_starknet(
            args, raise_on_nonzero=True, gateway_url=feeder_gateway_url
        )
        return json.loads(output.stdout)

    return run_starknet(args, raise_on_nonzero=False, gateway_url=feeder_gateway_url)


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
    total_blocks_str = re.search(
        "There are currently (.*) blocks.", error_message
    ).group(1)
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

    assert_equal(
        latest_block["sequencer_address"], hex(DEFAULT_GENERAL_CONFIG.sequencer_address)
    )
    assert_equal(latest_block["gas_price"], hex(DEFAULT_GENERAL_CONFIG.min_gas_price))
    assert re.match(r"^[a-fA-F0-9]{64}$", latest_block["state_root"])


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


def mint(address: str, amount: int, lite=False):
    """Sends mint request; returns parsed json body"""
    response = requests.post(
        f"{APP_URL}/mint", json={"address": address, "amount": amount, "lite": lite}
    )
    assert response.status_code == 200
    return response.json()


class DevnetBackgroundProc:
    """Helper for ensuring we always have only 1 active devnet server running in background"""

    def __init__(self):
        self.proc = None

    def start(self, *args, stderr=None, stdout=None, env=None):
        """Starts a new devnet-server instance. Previously active instance will be stopped."""
        self.stop()
        self.proc = run_devnet_in_background(
            *args, stderr=stderr, stdout=stdout, env=env
        )
        return self.proc

    def stop(self):
        """Stops the currently active devnet-server instance"""
        if self.proc:
            terminate_and_wait(self.proc)
            self.proc = None


def read_stream(stream: IO, encoding="utf-8") -> str:
    """Return stdout and stderr of `proc`"""
    return stream.read().decode(encoding)


class ErrorExpector:
    """
    Use this wrapper to assert that a block of code will raise
    a ReturnCodeAssertionError with the expected exception type
    """

    def __init__(self, expected_exc_type: Exception):
        self.expected_exc_type = expected_exc_type

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        if not exc_type:
            pytest.fail(f"Should have failed with {self.expected_exc_type}")

        if exc_type is ReturnCodeAssertionError:
            assert str(self.expected_exc_type) in str(exc_val)
            return True

        return False


def demand_block_creation():
    """Demand block creation. Useful when devnet started with --blocks-on-demand"""
    return requests.post(f"{APP_URL}/create_block")


def increase_time(time_s):
    """Increases the block timestamp offset"""
    increase_time_response = requests.post(
        f"{APP_URL}/increase_time", json={"time": time_s}
    )

    if increase_time_response.status_code == 200:
        assert increase_time_response.json().get("timestamp_increased_by") == time_s

    return increase_time_response


def set_time(time_s):
    """Sets the block timestamp and offset"""
    set_time_response = requests.post(f"{APP_URL}/set_time", json={"time": time_s})

    if set_time_response == 200:
        assert set_time_response.json().get("block_timestamp") == time_s

    return set_time_response


def assert_undeclared_class(resp=requests.Response):
    """Assert that the provided response indicates a failure due to an undeclared class"""
    assert resp.status_code == 500, resp.json()
    resp_body = resp.json()
    assert "code" in resp_body
    assert resp_body["code"] == str(StarknetErrorCode.UNDECLARED_CLASS)
