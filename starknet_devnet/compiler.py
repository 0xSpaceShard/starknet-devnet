"""Compilation utilities"""

import json
import os
import subprocess
import tempfile
from abc import ABC

from starkware.starknet.definitions.error_codes import StarknetErrorCode
from starkware.starknet.services.api.contract_class.contract_class import (
    CompiledClass,
    ContractClass,
)
from starkware.starknet.services.api.contract_class.contract_class_utils import (
    compile_contract_class,
)

from starknet_devnet.util import StarknetDevnetException


class ContractClassCompiler(ABC):
    """Base class of contract class compilers"""

    def compile_contract_class(self, contract_class: ContractClass) -> CompiledClass:
        """Take the sierra and return the compiled instance"""
        raise NotImplementedError


class DefaultContractClassCompiler(ContractClassCompiler):
    """Uses the default internal cairo-lang compiler"""

    def compile_contract_class(self, contract_class: ContractClass) -> CompiledClass:
        return compile_contract_class(contract_class)


class CustomContractClassCompiler(ContractClassCompiler):
    """Uses the compiler according to the compiler_manifest provided in initialization"""

    def __init__(self, compiler_manifest: str):
        self.compiler_manifest = compiler_manifest

    def compile_contract_class(self, contract_class: ContractClass) -> CompiledClass:

        with tempfile.TemporaryDirectory() as tmp_dir:
            contract_json = os.path.join(tmp_dir, "contract.json")
            contract_casm = os.path.join(tmp_dir, "contract.casm")

            with open(contract_json, mode="w", encoding="utf-8") as tmp_file:
                contract_class_dumped = contract_class.dump()
                contract_class_dumped["abi"] = json.loads(contract_class_dumped["abi"])
                json.dump(contract_class_dumped, tmp_file)

            compilation_args = [
                "cargo",
                "run",
                "--bin",
                "starknet-sierra-compile",
                "--manifest-path",
                self.compiler_manifest,
                "--",
                "--allowed-libfuncs-list-name",
                "experimental_v0.1.0",
                "--add-pythonic-hints",
                contract_json,
                contract_casm,
            ]
            compilation = subprocess.run(
                compilation_args, capture_output=True, check=False
            )
            if compilation.returncode:
                stderr = compilation.stderr.decode("utf-8")
                raise StarknetDevnetException(
                    code=StarknetErrorCode.UNEXPECTED_FAILURE,
                    message=f"Failed compilation to casm! {stderr}",
                )

            with open(contract_casm, encoding="utf-8") as casm_file:
                compiled_class = CompiledClass.loads(casm_file.read())
            return compiled_class
