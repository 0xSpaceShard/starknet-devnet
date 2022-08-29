"""
Contains the server implementation and its utility classes and functions.
"""
import sys
from starkware.crypto.signature.fast_pedersen_hash import pedersen_hash
from crypto_cpp_py.cpp_bindings import cpp_hash


__version__ = "0.2.11"


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
