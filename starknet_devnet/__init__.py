"""
Contains the server implementation and its utility classes and functions.
"""

import os
import sys
from copy import copy

import starkware.cairo.lang.vm.crypto
from crypto_cpp_py.cpp_bindings import cpp_hash
from starkware.crypto.signature.fast_pedersen_hash import pedersen_hash
from starkware.starknet.services.api.contract_class import ContractClass

from .util import warn

__version__ = "0.4.3"


def patched_pedersen_hash(left: int, right: int) -> int:
    """
    Pedersen hash function written in c++
    """
    return cpp_hash(left, right)


# This is a monkey-patch to improve the performance of the devnet
# We are using c++ code for calculating the pedersen hashes
# instead of python implementation from cairo-lang package
setattr(
    sys.modules["starkware.crypto.signature.fast_pedersen_hash"],
    "pedersen_hash",
    patched_pedersen_hash,
)
setattr(
    sys.modules["starkware.cairo.lang.vm.crypto"],
    "pedersen_hash",
    patched_pedersen_hash,
)


# Deep copy of a ContractClass takes a lot of time, but it should never be mutated.
def simpler_copy(self, memo):  # pylint: disable=unused-argument
    """
    A dummy implementation of ContractClass.__deepcopy__
    """
    return copy(self)


setattr(ContractClass, "__deepcopy__", simpler_copy)


# Optionally apply cairo-rs-py monkey patch
_VM_VAR = "STARKNET_DEVNET_CAIRO_VM"
_cairo_vm = os.environ.get(_VM_VAR)
if _cairo_vm == "rust":
    from starknet_devnet.cairo_rs_py_patch import cairo_rs_py_monkeypatch

    cairo_rs_py_monkeypatch()
    warn("Using Cairo VM: Rust")

elif not _cairo_vm or _cairo_vm == "python":
    # python VM set by default
    pass

else:
    sys.exit(f"Error: Invalid value of environment variable {_VM_VAR}: '{_cairo_vm}'")
