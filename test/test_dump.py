"""
Test server state serialization (dumping/loading).
"""

from asyncio import subprocess
import os
import signal
import time
import requests

import pytest

from .test_account import get_account_balance
from .test_fee_token import mint
from .util import (
    call,
    deploy,
    devnet_in_background,
    invoke,
    run_devnet_in_background,
    terminate_and_wait,
)
from .settings import APP_URL
from .shared import (
    CONTRACT_PATH,
    ABI_PATH,
)

DUMP_PATH = "dump.pkl"


class DevnetBackgroundProc:
    """Helper for ensuring we always have only 1 active devnet server running in background"""

    def __init__(self):
        self.proc = None

    def start(self, *args, stderr=None, stdout=None):
        """Starts a new devnet-server instance. Previously active instance will be stopped."""
        self.stop()
        self.proc = run_devnet_in_background(*args, stderr=stderr, stdout=stdout)
        return self.proc

    def stop(self):
        """Stops the currently active devnet-server instance"""
        if self.proc:
            terminate_and_wait(self.proc)
            self.proc = None


ACTIVE_DEVNET = DevnetBackgroundProc()


@pytest.fixture(autouse=True)
def run_before_and_after_test():
    """Cleanup after tests finish."""

    # before test
    ACTIVE_DEVNET.stop()

    yield

    # after test
    ACTIVE_DEVNET.stop()

    for path in os.listdir():
        if path.endswith(".pkl"):
            os.remove(path)


def send_dump_request(dump_path: str = None):
    """Send HTTP request to trigger dumping."""
    json_load = {"path": dump_path} if dump_path else None
    return requests.post(f"{APP_URL}/dump", json=json_load)


def send_load_request(load_path: str = None):
    """Send HTTP request to trigger loading."""
    json_load = {"path": load_path} if load_path else None
    return requests.post(f"{APP_URL}/load", json=json_load)


def send_error_request():
    """Send HTTP request to trigger error response."""
    json_body = {"dummy": "dummy_value"}
    return requests.post(f"{APP_URL}/dump", json=json_body)


def assert_dump_present(dump_path: str, sleep_seconds=2):
    """Assert there is a non-empty dump file."""
    time.sleep(sleep_seconds)
    assert os.path.isfile(dump_path)
    assert os.path.getsize(dump_path) > 0


def assert_no_dump_present(dump_path: str, sleep_seconds=2):
    """Assert there is no dump file."""
    time.sleep(sleep_seconds)
    assert not os.path.isfile(dump_path)


def dump_and_assert(dump_path: str = None):
    """Assert no dump file before dump and assert some dump file after dump."""
    assert_no_dump_present(dump_path)
    resp = send_dump_request(dump_path)
    assert resp.status_code == 200
    assert_dump_present(dump_path)


def assert_not_alive():
    """Assert devnet is not alive."""
    try:
        requests.get(f"{APP_URL}/is_alive")
        raise RuntimeError("Should have failed before this line.")
    except requests.exceptions.ConnectionError:
        pass


def deploy_empty_contract():
    """
    Deploy sample contract with balance = 0.
    Returns contract address.
    """
    deploy_dict = deploy(CONTRACT_PATH, inputs=["0"])
    contract_address = deploy_dict["address"]
    initial_balance = call("get_balance", contract_address, ABI_PATH)
    assert initial_balance == "0"
    return contract_address


def test_load_via_cli_if_no_file():
    """Test loading via CLI if dump file not present."""
    assert_no_dump_present(DUMP_PATH)
    devnet_proc = ACTIVE_DEVNET.start(
        "--load-path", DUMP_PATH, "--accounts", "0", stderr=subprocess.PIPE
    )
    assert devnet_proc.returncode == 1
    expected_msg = f"Error: Cannot load from {DUMP_PATH}. Make sure the file exists and contains a Devnet dump.\n"
    assert expected_msg == devnet_proc.stderr.read().decode("utf-8")


def test_mint_after_load():
    """Assert that minting can be done after loading."""
    devnet_proc = ACTIVE_DEVNET.start("--dump-path", DUMP_PATH, "--dump-on", "exit")
    dummy_address = "0x1"
    initial_balance = get_account_balance(dummy_address)
    assert initial_balance == 0

    terminate_and_wait(devnet_proc)
    assert_dump_present(DUMP_PATH)

    loaded_devnet_proc = ACTIVE_DEVNET.start("--load-path", DUMP_PATH)
    dummy_amount = 1
    resp_body = mint(dummy_address, dummy_amount)
    assert resp_body["new_balance"] == dummy_amount

    final_balance = get_account_balance(dummy_address)
    assert final_balance == dummy_amount

    terminate_and_wait(loaded_devnet_proc)


@devnet_in_background()
def test_load_via_http_if_no_file():
    """Test loading via HTTP if dump file not present."""
    assert_no_dump_present(DUMP_PATH)

    resp = send_load_request(load_path=DUMP_PATH)
    expected_msg = f"Error: Cannot load from {DUMP_PATH}. Make sure the file exists and contains a Devnet dump."
    assert resp.json()["message"] == expected_msg
    assert resp.status_code == 400


@devnet_in_background()
def test_dumping_if_path_not_provided():
    """Assert failure if dumping attempted without a known path."""
    resp = send_dump_request()
    assert resp.status_code == 400


NONEXISTENT_DIR = "nonexistent-dir"


def test_dumping_if_nonexistent_dir_via_cli():
    """Assert failure if dumping attempted via cli with a path containing a nonexistent dir"""
    invalid_path = os.path.join(NONEXISTENT_DIR, DUMP_PATH)
    devnet_proc = ACTIVE_DEVNET.start(
        "--dump-path", invalid_path, "--accounts", "0", stderr=subprocess.PIPE
    )
    assert devnet_proc.returncode == 1

    expected_msg = f"Invalid dump path: directory '{NONEXISTENT_DIR}' not found.\n"
    assert expected_msg == devnet_proc.stderr.read().decode("utf-8")


@devnet_in_background()
def test_dumping_if_nonexistent_dir_via_http():
    """Assert failure if dumping attempted via http with a path containing a nonexistent dir"""
    invalid_path = os.path.join(NONEXISTENT_DIR, DUMP_PATH)

    resp = send_dump_request(dump_path=invalid_path)
    assert (
        resp.json()["message"]
        == f"Invalid dump path: directory '{NONEXISTENT_DIR}' not found."
    )
    assert resp.status_code == 400


@devnet_in_background("--dump-path", DUMP_PATH)
def test_dumping_if_path_provided_as_cli_option():
    """Test dumping if path provided as CLI option"""
    resp = send_dump_request()
    assert resp.status_code == 200
    assert_dump_present(DUMP_PATH)


def test_loading_via_cli():
    """Test dumping via endpoint and loading via CLI."""
    # init devnet + contract
    ACTIVE_DEVNET.start()
    contract_address = deploy_empty_contract()

    invoke(
        function="increase_balance",
        inputs=["10", "20"],
        address=contract_address,
        abi_path=ABI_PATH,
    )
    balance_after_invoke = call("get_balance", contract_address, ABI_PATH)
    assert balance_after_invoke == "30"

    dump_and_assert(DUMP_PATH)

    ACTIVE_DEVNET.stop()
    assert_not_alive()

    # spawn new devnet and load path through CLI
    ACTIVE_DEVNET.start("--load-path", DUMP_PATH)

    loaded_balance = call("get_balance", contract_address, ABI_PATH)
    assert loaded_balance == balance_after_invoke

    # assure that new invokes can be made
    invoke(
        function="increase_balance",
        address=contract_address,
        inputs=["15", "25"],
        abi_path=ABI_PATH,
    )
    balance_after_invoke_on_loaded = call(
        "get_balance", contract_address, abi_path=ABI_PATH
    )
    assert balance_after_invoke_on_loaded == "70"

    os.remove(DUMP_PATH)
    ACTIVE_DEVNET.stop()
    assert_no_dump_present(DUMP_PATH)


def test_dumping_and_loading_via_endpoint():
    """Test dumping and loading via endpoint."""
    # init devnet + contract
    ACTIVE_DEVNET.start()
    contract_address = deploy_empty_contract()

    invoke("increase_balance", ["10", "20"], contract_address, ABI_PATH)
    balance_after_invoke = call("get_balance", contract_address, ABI_PATH)
    assert balance_after_invoke == "30"

    dump_and_assert(DUMP_PATH)

    ACTIVE_DEVNET.stop()
    assert_not_alive()

    # spawn new devnet and load path via endpoint call
    ACTIVE_DEVNET.start()
    send_load_request(DUMP_PATH)

    loaded_balance = call("get_balance", contract_address, ABI_PATH)
    assert loaded_balance == balance_after_invoke

    # assure that new invokes can be made
    invoke("increase_balance", ["15", "25"], contract_address, ABI_PATH)
    balance_after_invoke_on_loaded = call(
        "get_balance", contract_address, abi_path=ABI_PATH
    )
    assert balance_after_invoke_on_loaded == "70"

    os.remove(DUMP_PATH)
    ACTIVE_DEVNET.stop()
    assert_no_dump_present(DUMP_PATH)


def test_dumping_on_exit():
    """Test dumping on exit."""
    devnet_proc = ACTIVE_DEVNET.start("--dump-on", "exit", "--dump-path", DUMP_PATH)

    contract_address = deploy_empty_contract()

    invoke("increase_balance", ["10", "20"], contract_address, ABI_PATH)
    balance_after_invoke = call("get_balance", contract_address, ABI_PATH)
    assert balance_after_invoke == "30"

    assert_no_dump_present(DUMP_PATH)
    devnet_proc.send_signal(
        signal.SIGINT
    )  # send SIGINT because devnet doesn't handle SIGKILL
    assert_dump_present(DUMP_PATH, sleep_seconds=3)


def test_invalid_dump_on_option():
    """Test behavior when invalid dump-on is provided."""
    devnet_proc = ACTIVE_DEVNET.start(
        "--dump-on",
        "obviously-invalid",
        "--dump-path",
        DUMP_PATH,
        stderr=subprocess.PIPE,
    )

    assert devnet_proc.returncode == 1
    expected_msg = b"Error: Invalid --dump-on option: obviously-invalid. Valid options: exit, transaction\n"
    assert devnet_proc.stderr.read() == expected_msg


def test_dump_path_not_present_with_dump_on_present():
    """Test behavior when dump-path is not present and dump-on is."""
    devnet_proc = ACTIVE_DEVNET.start("--dump-on", "exit", stderr=subprocess.PIPE)

    assert devnet_proc.returncode == 1
    expected_msg = b"Error: --dump-path required if --dump-on present\n"
    assert devnet_proc.stderr.read() == expected_msg


def assert_load(dump_path: str, contract_address: str, expected_value: str):
    """Load from `dump_path` and assert get_balance at `contract_address` returns `expected_value`."""

    ACTIVE_DEVNET.start("--load-path", dump_path)
    assert call("get_balance", contract_address, ABI_PATH) == expected_value
    ACTIVE_DEVNET.stop()
    os.remove(dump_path)


def test_dumping_on_each_tx():
    """Test dumping on each transaction."""
    ACTIVE_DEVNET.start("--dump-on", "transaction", "--dump-path", DUMP_PATH)

    # deploy
    contract_address = deploy_empty_contract()
    assert_dump_present(DUMP_PATH)
    dump_after_deploy_path = "dump_after_deploy.pkl"
    os.rename(DUMP_PATH, dump_after_deploy_path)

    # invoke
    invoke("increase_balance", ["5", "5"], contract_address, ABI_PATH)
    assert_dump_present(DUMP_PATH)
    dump_after_invoke_path = "dump_after_invoke.pkl"
    os.rename(DUMP_PATH, dump_after_invoke_path)

    ACTIVE_DEVNET.stop()

    assert_load(dump_after_deploy_path, contract_address, "0")
    assert_load(dump_after_invoke_path, contract_address, "10")


@devnet_in_background()
def test_dumping_call_with_invalid_body():
    """Call with invalid body and test status code and message."""
    resp = send_error_request()

    json_error_message = resp.json()["message"]
    msg = "No path provided."
    assert msg == json_error_message
    assert resp.status_code == 400
