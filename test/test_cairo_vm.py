"""Test specifying cairo VM"""

import os

import subprocess
import pytest
from .util import (
    DevnetBackgroundProc,
    read_stream,
    terminate_and_wait,
)

ACTIVE_DEVNET = DevnetBackgroundProc()


@pytest.mark.parametrize(
    "cairo_vm",
    ["", None, "python", "rust"],
)
def test_valid_cairo_vm(cairo_vm):
    """Test if the invalid chain id fails"""

    if cairo_vm is None:
        del os.environ["STARKNET_DEVNET_CAIRO_VM"]
    else:
        os.environ["STARKNET_DEVNET_CAIRO_VM"] = cairo_vm

    proc = ACTIVE_DEVNET.start(stderr=subprocess.PIPE)
    terminate_and_wait(proc)
    assert proc.returncode == 0


@pytest.mark.parametrize(
    "cairo_vm",
    ["invalid_value", " rust"],
)
def test_invalid_cairo_vm(cairo_vm):
    """Test random invalid cairo vm specifications"""

    os.environ["STARKNET_DEVNET_CAIRO_VM"] = cairo_vm
    proc = ACTIVE_DEVNET.start(stderr=subprocess.PIPE)

    terminate_and_wait(proc)
    assert (
        f"Error: Invalid value of environment variable STARKNET_DEVNET_CAIRO_VM: '{cairo_vm}'"
        in read_stream(proc.stderr)
    )
    assert proc.returncode == 1
