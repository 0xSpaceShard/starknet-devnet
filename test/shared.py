"""Shared values between tests"""

import json

from starkware.starknet.third_party.open_zeppelin.starknet_contracts import (
    account_contract as oz_account_class,
)

ARTIFACTS_PATH = "test/artifacts/contracts/cairo"
CONTRACT_PATH = f"{ARTIFACTS_PATH}/contract.cairo/contract.json"
ABI_PATH = f"{ARTIFACTS_PATH}/contract.cairo/contract_abi.json"
STORAGE_CONTRACT_PATH = f"{ARTIFACTS_PATH}/storage.cairo/storage.json"
STORAGE_ABI_PATH = f"{ARTIFACTS_PATH}/storage.cairo/storage_abi.json"
EVENTS_CONTRACT_PATH = f"{ARTIFACTS_PATH}/events.cairo/events.json"
EVENTS_ABI_PATH = f"{ARTIFACTS_PATH}/events.cairo/events_abi.json"
FAILING_CONTRACT_PATH = f"{ARTIFACTS_PATH}/always_fail.cairo/always_fail.json"
DEPLOYER_CONTRACT_PATH = f"{ARTIFACTS_PATH}/deployer.cairo/deployer.json"
DEPLOYER_ABI_PATH = f"{ARTIFACTS_PATH}/deployer.cairo/deployer_abi.json"
REPLACEABLE_CONTRACT_PATH = f"{ARTIFACTS_PATH}/replaceable.cairo/replaceable.json"
REPLACEABLE_ABI_PATH = f"{ARTIFACTS_PATH}/replaceable.cairo/replaceable_abi.json"
REPLACING_CONTRACT_PATH = f"{ARTIFACTS_PATH}/replacing.cairo/replacing.json"
REPLACING_ABI_PATH = f"{ARTIFACTS_PATH}/replacing.cairo/replacing_abi.json"

# cairo 1.0
ARTIFACTS_1_PATH = "test/artifacts/contracts/cairo1"
CONTRACT_1_PATH = f"{ARTIFACTS_1_PATH}/contract.cairo/contract.json"
CONTRACT_1_CASM_PATH = f"{ARTIFACTS_1_PATH}/contract.cairo/contract.casm"
ABI_1_PATH = f"{ARTIFACTS_1_PATH}/contract.cairo/contract_abi.json"

STARKNET_CLI_ACCOUNT_ABI_PATH = f"{ARTIFACTS_PATH}/starknet_cli_oz_account_abi.json"
with open(STARKNET_CLI_ACCOUNT_ABI_PATH, "w", encoding="utf-8") as oz_account_abi_file:
    json.dump(oz_account_class.abi, oz_account_abi_file)

L1L2_CONTRACT_PATH = f"{ARTIFACTS_PATH}/l1l2.cairo/l1l2.json"
L1L2_ABI_PATH = f"{ARTIFACTS_PATH}/l1l2.cairo/l1l2_abi.json"

BALANCE_KEY = (
    "916907772491729262376534102982219947830828984996257231353398618781993312401"
)

EXPECTED_CLASS_HASH = (
    "0x2216379233d6f7c8890a730e164e069dafdd63edf0f0218dde8181b50fabd45"
)
# hash of contract at CONTRACT_1_PATH
EXPECTED_CLASS_1_HASH = (
    "0x7c93b555d40b18537ae7107aaa26dce3f8658337f5be3f563955fd24b623fec"
)

NONEXISTENT_TX_HASH = "0x99"
GENESIS_BLOCK_NUMBER = 0
GENESIS_BLOCK_HASH = "0x0"
INCORRECT_GENESIS_BLOCK_HASH = "0x1"

SUPPORTED_TX_VERSION = 1
SUPPORTED_RPC_TX_VERSION = 1

PREDEPLOY_ACCOUNT_CLI_ARGS = ("--seed", "42", "--accounts", "1")
PREDEPLOYED_ACCOUNT_ADDRESS = (
    "0x347be35996a21f6bf0623e75dbce52baba918ad5ae8d83b6f416045ab22961a"
)
PREDEPLOYED_ACCOUNT_PRIVATE_KEY = 0xBDD640FB06671AD11C80317FA3B1799D

EXPECTED_FEE_TOKEN_ADDRESS = (
    "0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7"
)
EXPECTED_UDC_ADDRESS = (
    "0x41a78e741e5af2fec34b695679bc6891742439f7afb8484ecd7766661ad02bf"
)
FEE_CHARGED_EVENT_KEY = (
    "0x99cd8bde557814842a3121e8ddfd433a539b8c9f14bf31ebf108d12e6196e9"
)
INCREASE_BALANCE_CALLED_EVENT_KEY = (
    "0x3db3da4221c078e78bd987e54e1cc24570d89a7002cefa33e548d6c72c73f9d"
)

ALPHA_MAINNET_URL = "https://alpha-mainnet.starknet.io"
ALPHA_GOERLI_URL = "https://alpha4.starknet.io"
ALPHA_GOERLI2_URL = "https://alpha4-2.starknet.io"

# useful to be provided if we want to avoid implicit estimation
SUFFICIENT_MAX_FEE = int(1e18)
