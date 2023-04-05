"""Script for computing compiled class hash of a contract"""

import sys

from starkware.starknet.core.os.contract_class.deprecated_class_hash import (
    compute_deprecated_class_hash,
)
from starkware.starknet.services.api.contract_class.contract_class import (
    DeprecatedCompiledClass,
)


def main():
    """Main function"""
    casm_path = sys.argv[1]
    with open(casm_path, encoding="utf-8") as casm_file:
        casm = DeprecatedCompiledClass.loads(casm_file.read())
    compiled_class_hash = compute_deprecated_class_hash(casm)
    print(hex(compiled_class_hash))


if __name__ == "__main__":
    main()
