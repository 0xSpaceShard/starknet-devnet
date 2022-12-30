"""Test specifying cairo VM"""

import os
import subprocess

import pytest

from .util import DevnetBackgroundProc, read_stream, terminate_and_wait

ACTIVE_DEVNET = DevnetBackgroundProc()

_VM_VAR = "STARKNET_DEVNET_CAIRO_VM"
_RUST_VM_LOG_LINE = "Using Cairo VM: Rust"


@pytest.mark.parametrize(
    "cairo_vm, assert_rust_vm_logged",
    [
        ("", False),
        ("python", False),
        ("rust", True),
    ],
)
def test_valid_cairo_vm(cairo_vm, assert_rust_vm_logged):
    """Test if the invalid chain id fails"""

    env_copy = os.environ.copy()
    env_copy[_VM_VAR] = cairo_vm

    proc = ACTIVE_DEVNET.start(stderr=subprocess.PIPE, env=env_copy)
    terminate_and_wait(proc)

    stderr = proc.stderr.read().decode("utf-8")
    if assert_rust_vm_logged:
        assert _RUST_VM_LOG_LINE in stderr
    else:
        assert _RUST_VM_LOG_LINE not in stderr

    assert proc.returncode == 0


def test_passing_if_no_cairo_vm_set():
    """If no vm env var set, it should assume python and pass"""
    env_copy = os.environ.copy()
    if _VM_VAR in env_copy:
        del env_copy[_VM_VAR]

    proc = ACTIVE_DEVNET.start(stderr=subprocess.PIPE, env=env_copy)
    terminate_and_wait(proc)

    assert _RUST_VM_LOG_LINE not in proc.stderr.read().decode("utf-8")
    assert proc.returncode == 0


@pytest.mark.parametrize("cairo_vm", ["invalid_value", " rust"])
def test_invalid_cairo_vm(cairo_vm):
    """Test random invalid cairo vm specifications"""

    env_copy = os.environ.copy()
    env_copy[_VM_VAR] = cairo_vm
    proc = ACTIVE_DEVNET.start(stderr=subprocess.PIPE, env=env_copy)

    terminate_and_wait(proc)
    assert (
        f"Error: Invalid value of environment variable {_VM_VAR}: '{cairo_vm}'"
        in read_stream(proc.stderr)
    )
    assert proc.returncode == 1
