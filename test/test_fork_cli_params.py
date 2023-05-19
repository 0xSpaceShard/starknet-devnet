"""Testing fork CLI params"""

import subprocess

import pytest

from .shared import ALPHA_GOERLI2_URL, ALPHA_GOERLI_URL, ALPHA_MAINNET_URL
from .util import DevnetBackgroundProc, read_stream, terminate_and_wait

ACTIVE_DEVNET = DevnetBackgroundProc()


def test_invalid_fork_network():
    """Test if fork network invalid"""
    invalid_name = "alpha-goerli-invalid"
    proc = ACTIVE_DEVNET.start(
        "--fork-network",
        invalid_name,
        stderr=subprocess.PIPE,
        stdout=subprocess.PIPE,
    )
    assert read_stream(proc.stdout) == ""
    assert (
        f"Error: Invalid fork-network (must be a URL or one of {{alpha-goerli, alpha-goerli2, alpha-mainnet}}). Received: {invalid_name}\n"
        in read_stream(proc.stderr)
    )
    assert proc.returncode == 1


def test_url_not_sequencer():
    """Pass a valid url but not of a Starknet sequencer"""
    invalid_url = "http://google.com"
    proc = ACTIVE_DEVNET.start(
        "--fork-network",
        invalid_url,
        stderr=subprocess.PIPE,
        stdout=subprocess.PIPE,
    )
    assert read_stream(proc.stdout) == ""
    assert f"Error: {invalid_url} is not a valid Starknet sequencer\n" in read_stream(
        proc.stderr
    )
    assert proc.returncode == 1


@pytest.mark.parametrize(
    "fork_network, expected_stdout",
    [
        ("alpha-mainnet", f"Forking {ALPHA_MAINNET_URL}"),
        (ALPHA_MAINNET_URL, f"Forking {ALPHA_MAINNET_URL}"),
        ("alpha-goerli", f"Forking {ALPHA_GOERLI_URL}"),
        (ALPHA_GOERLI_URL, f"Forking {ALPHA_GOERLI_URL}"),
        ("alpha-goerli2", f"Forking {ALPHA_GOERLI2_URL}"),
        (ALPHA_GOERLI2_URL, f"Forking {ALPHA_GOERLI2_URL}"),
    ],
)
def test_predefined_fork_network_specification(
    fork_network: str,
    expected_stdout: str,
):
    """Test various happy path fork network specification scenarios"""
    proc = ACTIVE_DEVNET.start(
        "--accounts",
        "0",  # to reduce output
        "--fork-network",
        fork_network,
        stdout=subprocess.PIPE,
    )
    terminate_and_wait(proc)
    assert expected_stdout in read_stream(proc.stdout)
    assert proc.returncode == 0


def test_block_provided_without_network():
    """Should fail if block provided and network not"""
    proc = ACTIVE_DEVNET.start(
        "--fork-block", "123", stderr=subprocess.PIPE, stdout=subprocess.PIPE
    )
    assert read_stream(proc.stdout) == ""
    assert "Error: --fork-network required if --fork-block present\n" in read_stream(
        proc.stderr
    )
    assert proc.returncode == 1


@pytest.mark.parametrize("fork_block", ["-1", "piece of invalid text"])
def test_malformed_block_id(fork_block: str):
    """Should exit if provided with a negative block number"""
    proc = ACTIVE_DEVNET.start(
        "--fork-network",
        "alpha-goerli",
        "--fork-block",
        fork_block,
        stderr=subprocess.PIPE,
        stdout=subprocess.PIPE,
    )
    assert read_stream(proc.stdout) == ""
    assert (
        f"The value of --fork-block must be a non-negative integer or 'latest', got: {fork_block}\n"
        in read_stream(proc.stderr)
    )

    assert proc.returncode == 1


def test_too_big_block_id():
    """Should exit if fork block number too big"""
    too_big_block_id = str(int(1e9))
    proc = ACTIVE_DEVNET.start(
        "--fork-network",
        "alpha-goerli2",
        "--fork-block",
        too_big_block_id,
        stderr=subprocess.PIPE,
        stdout=subprocess.PIPE,
    )
    assert read_stream(proc.stdout) == ""
    assert f"Block number {too_big_block_id} was not found." in read_stream(proc.stderr)
    assert proc.returncode != 0


@pytest.mark.parametrize(
    "fork_block",
    [
        "latest",  # would be hard to assert the block number is correct
        "1",  # small enough, every chain should have it
    ],
)
def test_valid_block_ids(fork_block: str):
    """Test some happy path fork block ids"""
    proc = ACTIVE_DEVNET.start(
        "--fork-network",
        "alpha-goerli2",
        "--fork-block",
        fork_block,
        stdout=subprocess.PIPE,
    )
    terminate_and_wait(proc)
    assert f"Forking {ALPHA_GOERLI2_URL}" in read_stream(proc.stdout)
    assert proc.returncode == 0


@pytest.mark.parametrize("fork_retries", ["-1", "0"])
def test_out_of_range_fork_retries(fork_retries: str):
    """Should exit if provided with a negative block number"""
    proc = ACTIVE_DEVNET.start(
        "--fork-network",
        "alpha-goerli",
        "--fork-retries",
        fork_retries,
        stderr=subprocess.PIPE,
        stdout=subprocess.PIPE,
    )
    assert read_stream(proc.stdout) == ""
    assert (
        f"error: argument --fork-retries must be a positive integer; got: {fork_retries}.\n"
        in read_stream(proc.stderr)
    )

    assert proc.returncode == 2


def test_invalid_fork_retries():
    """Should exit if provided with non integer"""
    fork_retries = "invalid"

    proc = ACTIVE_DEVNET.start(
        "--fork-network",
        "alpha-goerli",
        "--fork-retries",
        fork_retries,
        stderr=subprocess.PIPE,
        stdout=subprocess.PIPE,
    )
    assert read_stream(proc.stdout) == ""
    assert (
        f"error: argument --fork-retries: invalid int value: '{fork_retries}'\n"
        in read_stream(proc.stderr)
    )

    assert proc.returncode == 2


@pytest.mark.parametrize(
    "fork_retries",
    [
        "1",
        "3",
    ],
)
def test_valid_fork_retries(fork_retries: str):
    """Test some happy path fork retries values"""
    proc = ACTIVE_DEVNET.start(
        "--fork-network",
        "alpha-goerli2",
        "--fork-retries",
        fork_retries,
        stdout=subprocess.PIPE,
    )
    terminate_and_wait(proc)
    assert f"Forking {ALPHA_GOERLI2_URL}" in read_stream(proc.stdout)
    assert proc.returncode == 0
