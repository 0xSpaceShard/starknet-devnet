"""Testing chain id CLI params"""

import subprocess

import pytest
from starkware.starknet.definitions.general_config import StarknetChainId

from .util import DevnetBackgroundProc, read_stream

ACTIVE_DEVNET = DevnetBackgroundProc()

@pytest.mark.parametrize(
    "chain_id",
    [member.name for member in StarknetChainId],
)
def test_chain_id_valid(chain_id):
    """Test if chain id works"""
    proc = ACTIVE_DEVNET.start(
        "--chain-id",
        chain_id,
    )
    assert proc.returncode is None

def test_chain_id_invalid():
    """Test if the invalid chain id fails"""
    proc = ACTIVE_DEVNET.start(
        "--chain-id",
        "",
        stderr=subprocess.PIPE,
        stdout=subprocess.PIPE,
    )
    assert (
        f"The value of --chain_id must be in {[member.name for member in StarknetChainId]}, got:"
        in read_stream(proc.stderr)
    )
    assert proc.returncode == 1
