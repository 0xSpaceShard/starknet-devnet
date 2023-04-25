"""
Contains the server implementation and its utility classes and functions.
This file contains monkeypatches used across the project. Advice for monkeypatch atomicity:
- Define a patching function
    - The function should import the places to be patched
    - The function can define the implementation to use for overwriting
- Call the patching function
"""

# pylint: disable=unused-import
# pylint: disable=import-outside-toplevel

import os
import sys

__version__ = "0.5.0"


def _patch_pedersen_hash():
    """
    This is a monkey-patch to improve the performance of the devnet
    We are using c++ code for calculating the pedersen hashes
    instead of python implementation from cairo-lang package
    """

    from crypto_cpp_py.cpp_bindings import cpp_hash as patched_pedersen_hash
    import starkware.crypto.signature.fast_pedersen_hash

    setattr(
        sys.modules["starkware.crypto.signature.fast_pedersen_hash"],
        "pedersen_hash",
        patched_pedersen_hash,
    )
    setattr(
        sys.modules["starkware.crypto.signature.fast_pedersen_hash"],
        "pedersen_hash_func",
        patched_pedersen_hash,
    )

    import starkware.cairo.lang.vm.crypto

    setattr(
        sys.modules["starkware.cairo.lang.vm.crypto"],
        "pedersen_hash",
        patched_pedersen_hash,
    )


_patch_pedersen_hash()


def _patch_poseidon_hash():
    """
    Improves performance by substituting the default Python implementation of Poseidon hash
    with swm's Python wrapper of a C implementation.
    """

    import starkware.cairo.common

    from poseidon_py import poseidon_hash

    # alternative, shorter approach
    # sys.modules["starkware.cairo.common"].poseidon_hash = poseidon_hash

    setattr(sys.modules["starkware.cairo.common"], "poseidon_hash", poseidon_hash)


_patch_poseidon_hash()  # doesn't have any effect if line 49 is active


def _patch_poseidon_hash_rust():
    """Uses equilibriums rust implementation of poseidon"""

    import starknet_pathfinder_crypto
    import starkware.crypto.signature.fast_pedersen_hash
    import starkware.cairo.common.poseidon_hash

    starkware.crypto.signature.fast_pedersen_hash.pedersen_hash_func = (
        starknet_pathfinder_crypto.pedersen_hash_func
    )
    starkware.crypto.signature.fast_pedersen_hash.pedersen_hash = (
        starknet_pathfinder_crypto.pedersen_hash
    )
    starkware.cairo.common.poseidon_hash.poseidon_hash = (
        starknet_pathfinder_crypto.poseidon_hash
    )
    starkware.cairo.common.poseidon_hash.poseidon_hash_func = (
        starknet_pathfinder_crypto.poseidon_hash_func
    )
    starkware.cairo.common.poseidon_hash.poseidon_hash_many = (
        starknet_pathfinder_crypto.poseidon_hash_many
    )
    starkware.cairo.common.poseidon_hash.poseidon_perm = (
        starknet_pathfinder_crypto.poseidon_perm
    )


# _patch_poseidon_hash_rust()


def _patch_copy():
    """Deep copy of a ContractClass takes a lot of time, but it should never be mutated."""

    from copy import copy

    from starkware.starknet.services.api.contract_class.contract_class import (
        CompiledClassBase,
        ContractClass,
    )

    def simpler_copy(self, memo):  # pylint: disable=unused-argument
        """
        A dummy implementation of __deepcopy__
        """
        return copy(self)

    setattr(ContractClass, "__deepcopy__", simpler_copy)
    setattr(CompiledClassBase, "__deepcopy__", simpler_copy)


_patch_copy()


def _patch_cairo_vm():
    """Apply cairo-rs-py monkey patch"""

    from starknet_devnet.cairo_rs_py_patch import cairo_rs_py_monkeypatch

    cairo_rs_py_monkeypatch()

    from .util import warn

    warn("Using Cairo VM: Rust")


_VM_VAR = "STARKNET_DEVNET_CAIRO_VM"
_cairo_vm = os.environ.get(_VM_VAR)

if _cairo_vm == "rust":
    _patch_cairo_vm()

elif not _cairo_vm or _cairo_vm == "python":
    # python VM set by default
    pass

else:
    sys.exit(f"Error: Invalid value of environment variable {_VM_VAR}: '{_cairo_vm}'")
