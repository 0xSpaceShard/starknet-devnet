"""Script for computing compiled class hash of a contract"""

import sys

from starkware.starknet.core.os.contract_class.compiled_class_hash import (
    compute_compiled_class_hash,
)
from starkware.starknet.services.api.contract_class.contract_class import CompiledClass


def main():
    """Main function"""
    casm_path = sys.argv[1]
    with open(casm_path, encoding="utf-8") as casm_file:
        casm = CompiledClass.loads(casm_file.read())
    compiled_class_hash = compute_compiled_class_hash(casm)
    print(hex(compiled_class_hash))


if __name__ == "__main__":
    main()
