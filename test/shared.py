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

STARKNET_CLI_ACCOUNT_ABI_PATH = f"{ARTIFACTS_PATH}/starknet_cli_oz_account_abi.json"
with open(STARKNET_CLI_ACCOUNT_ABI_PATH, "w", encoding="utf-8") as oz_account_abi_file:
    json.dump(oz_account_class.abi, oz_account_abi_file)

L1L2_CONTRACT_PATH = f"{ARTIFACTS_PATH}/l1l2.cairo/l1l2.json"
L1L2_ABI_PATH = f"{ARTIFACTS_PATH}/l1l2.cairo/l1l2_abi.json"

BALANCE_KEY = (
    "916907772491729262376534102982219947830828984996257231353398618781993312401"
)

SIGNATURE = [
    "1225578735933442828068102633747590437426782890965066746429241472187377583468",
    "3568809569741913715045370357918125425757114920266578211811626257903121825123",
]

EXPECTED_SALTY_DEPLOY_ADDRESS = (
    "0x01fbcb39359c1d204c9fa13318451c208026c01b77513d4eac72b13798184372"
)
EXPECTED_SALTY_DEPLOY_HASH = (
    "0x799357c54a299cfdc53cf56951cd6f80f8881bdf7fc1d220b92c0af45bf1302"
)

EXPECTED_CLASS_HASH = (
    "0x75dc4457c66bfb3808c67a918047b8f0973ea8b876056b72e99f651b91298ca"
)

NONEXISTENT_TX_HASH = "0x1"
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

ALPHA_MAINNET_URL = "https://alpha-mainnet.starknet.io"
ALPHA_GOERLI_URL = "https://alpha4.starknet.io"
ALPHA_GOERLI2_URL = "https://alpha4-2.starknet.io"
