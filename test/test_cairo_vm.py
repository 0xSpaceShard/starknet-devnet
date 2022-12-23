"""Test specifying cairo VM"""

import os
import subprocess

import pytest

from .util import DevnetBackgroundProc, read_stream, terminate_and_wait

ACTIVE_DEVNET = DevnetBackgroundProc()

_VM_VAR = "STARKNET_DEVNET_CAIRO_VM"


@pytest.mark.parametrize("cairo_vm", ["", "python", "rust"])
def test_valid_cairo_vm(cairo_vm):
    """Test if the invalid chain id fails"""

    env_copy = os.environ.copy()
    env_copy[_VM_VAR] = cairo_vm

    proc = ACTIVE_DEVNET.start(stderr=subprocess.PIPE, env=env_copy)
    terminate_and_wait(proc)
    assert proc.returncode == 0


def test_passing_if_no_cairo_vm_set():
    """If no vm env var set, it should assume python and pass"""
    env_copy = os.environ.copy()
    if _VM_VAR in env_copy:
        del env_copy[_VM_VAR]

    proc = ACTIVE_DEVNET.start(stderr=subprocess.PIPE, env=env_copy)
    terminate_and_wait(proc)
    assert proc.returncode == 0


@pytest.mark.parametrize("cairo_vm", ["invalid_value", " rust"])
def test_invalid_cairo_vm(cairo_vm):
    """Test random invalid cairo vm specifications"""

    env_copy = os.environ.copy()
    env_copy[_VM_VAR] = cairo_vm
    # TODO change testing stderr in other tests - add log for rust vm
    proc = ACTIVE_DEVNET.start(stderr=subprocess.PIPE, env=env_copy)

    terminate_and_wait(proc)
    assert (
        f"Error: Invalid value of environment variable {_VM_VAR}: '{cairo_vm}'"
        in read_stream(proc.stderr)
    )
    assert proc.returncode == 1
