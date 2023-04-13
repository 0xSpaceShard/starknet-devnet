"""
Account test functions and utilities.
Latest changes based on https://github.com/OpenZeppelin/nile/pull/184
"""

from typing import List, Tuple

import requests
from starkware.crypto.signature.signature import private_to_stark_key, sign
from starkware.starknet.cli.starknet_cli import get_salt
from starkware.starknet.core.os.contract_address.contract_address import (
    calculate_contract_address_from_hash,
)
from starkware.starknet.core.os.transaction_hash.transaction_hash import (
    calculate_declare_transaction_hash,
    calculate_deprecated_declare_transaction_hash,
)
from starkware.starknet.definitions.constants import QUERY_VERSION
from starkware.starknet.definitions.general_config import StarknetChainId
from starkware.starknet.services.api.gateway.transaction import ContractClass, Declare

from starknet_devnet.account_util import AccountCall, get_execute_args

from .settings import APP_URL
from .shared import EXPECTED_UDC_ADDRESS, SUPPORTED_TX_VERSION
from .util import deploy as deploy_contract
from .util import (
    estimate_fee,
    extract_class_hash,
    extract_tx_hash,
    load_contract_class,
    run_starknet,
)

ACCOUNT_ARTIFACTS_PATH = "starknet_devnet/accounts_artifacts"
ACCOUNT_AUTHOR = "OpenZeppelin"
ACCOUNT_VERSION = "0.5.1"

ACCOUNT_PATH = f"{ACCOUNT_ARTIFACTS_PATH}/{ACCOUNT_AUTHOR}/{ACCOUNT_VERSION}/Account.cairo/Account.json"
ACCOUNT_ABI_PATH = f"{ACCOUNT_ARTIFACTS_PATH}/{ACCOUNT_AUTHOR}/{ACCOUNT_VERSION}/Account.cairo/Account_abi.json"

PRIVATE_KEY = 123456789987654321
PUBLIC_KEY = private_to_stark_key(PRIVATE_KEY)


def deploy_account_contract(salt=None):
    """Deploy account contract."""
    return deploy_contract(ACCOUNT_PATH, inputs=[str(PUBLIC_KEY)], salt=salt)


def get_nonce(account_address: str, feeder_gateway_url=APP_URL) -> int:
    """Get nonce."""
    resp = requests.get(
        f"{feeder_gateway_url}/feeder_gateway/get_nonce?contractAddress={account_address}"
    )
    return int(resp.json(), 16)


def _get_signature(message_hash: int, private_key: int) -> Tuple[str, str]:
    """Get signature from message hash and private key."""
    sig_r, sig_s = sign(message_hash, private_key)
    return [str(sig_r), str(sig_s)]


def _adapt_inputs(execute_calldata: List[int]) -> List[str]:
    """Get stringified inputs from execute_calldata."""
    return [str(v) for v in execute_calldata]


def get_estimate_fee_request_dict(
    calls: List[AccountCall],
    account_address: str,
    private_key: str,
    nonce: int = None,
):
    """Create a mock tx to request fee estimation of an invoke"""
    if nonce is None:
        nonce = get_nonce(account_address)

    max_fee = 0
    signature, execute_calldata = get_execute_args(
        calls=calls,
        account_address=account_address,
        private_key=private_key,
        nonce=nonce,
        max_fee=max_fee,
        version=QUERY_VERSION,
    )

    return {
        "contract_address": account_address,
        "max_fee": hex(max_fee),
        "calldata": [str(element) for element in execute_calldata],
        "version": hex(QUERY_VERSION),
        "nonce": hex(nonce),
        "signature": signature,
        "type": "INVOKE_FUNCTION",
    }


# pylint: disable=too-many-arguments
def get_estimated_fee(
    calls: List[AccountCall],
    account_address: str,
    private_key: str,
    nonce=None,
    feeder_gateway_url=APP_URL,
    block_number=None,
    chain_id=StarknetChainId.TESTNET,
):
    """Get estimated fee through account."""

    if nonce is None:
        nonce = get_nonce(account_address)

    signature, execute_calldata = get_execute_args(
        calls=calls,
        account_address=account_address,
        private_key=private_key,
        nonce=nonce,
        max_fee=0,
        version=QUERY_VERSION,
        chain_id=chain_id,
    )

    return estimate_fee(
        "__execute__",
        inputs=_adapt_inputs(execute_calldata),
        address=account_address,
        abi_path=ACCOUNT_ABI_PATH,
        signature=signature,
        nonce=nonce,
        block_number=block_number,
        feeder_gateway_url=feeder_gateway_url,
        chain_id=chain_id,
    )


def invoke(
    calls: List[AccountCall],
    account_address: str,
    private_key: int,
    nonce=None,
    max_fee=None,
    gateway_url=APP_URL,
    chain_id=StarknetChainId.TESTNET,
):
    """Invoke __execute__ with correct calldata and signature."""

    if nonce is None:
        nonce = get_nonce(account_address, feeder_gateway_url=gateway_url)

    if max_fee is None:
        max_fee = get_estimated_fee(
            calls=calls,
            account_address=account_address,
            private_key=private_key,
            nonce=nonce,
            feeder_gateway_url=gateway_url,
            chain_id=chain_id,
        )

    signature, execute_calldata = get_execute_args(
        calls=calls,
        account_address=account_address,
        private_key=private_key,
        nonce=nonce,
        version=SUPPORTED_TX_VERSION,
        max_fee=max_fee,
        chain_id=chain_id,
    )

    adapted_inputs = _adapt_inputs(execute_calldata)
    output = run_starknet(
        [
            "invoke",
            "--function",
            "__execute__",
            "--inputs",
            *adapted_inputs,
            "--address",
            account_address,
            "--abi",
            ACCOUNT_ABI_PATH,
            "--signature",
            *signature,
            "--max_fee",
            str(max_fee),
            "--chain_id",
            hex(chain_id.value),
        ],
        gateway_url=gateway_url,
    )

    print("Invoke sent!")
    return extract_tx_hash(output.stdout)


def declare(
    contract_path: str,
    account_address: str,
    private_key: str,
    nonce: int = None,
    max_fee: int = 0,
    chain_id=StarknetChainId.TESTNET,
):
    """Wrapper around starknet declare"""

    if nonce is None:
        nonce = get_nonce(account_address)

    tx_hash = calculate_deprecated_declare_transaction_hash(
        contract_class=load_contract_class(contract_path),
        chain_id=StarknetChainId.TESTNET.value,
        sender_address=int(account_address, 16),
        max_fee=max_fee,
        nonce=nonce,
        version=SUPPORTED_TX_VERSION,
    )
    signature = _get_signature(tx_hash, private_key)

    output = run_starknet(
        [
            "declare",
            "--deprecated",
            "--contract",
            contract_path,
            "--signature",
            *signature,
            "--sender",
            account_address,
            "--max_fee",
            str(max_fee),
            "--chain_id",
            hex(chain_id.value),
        ]
    )
    return {
        "tx_hash": extract_tx_hash(output.stdout),
        "class_hash": extract_class_hash(output.stdout),
    }


def deploy(
    class_hash: str,
    account_address: str,
    private_key: int,
    inputs=None,
    salt=None,
    unique=False,
    max_fee=None,
    gateway_url=APP_URL,
):
    """Wrapper around starknet deploy"""

    ctor_args = [int(val, 0) for val in inputs or []]
    salt = get_salt(salt)

    invoke_tx_hash = invoke(
        calls=[
            (
                EXPECTED_UDC_ADDRESS,
                "deployContract",
                [
                    int(class_hash, 16),
                    salt,
                    int(unique),
                    len(ctor_args),
                    *ctor_args,
                ],
            )
        ],
        account_address=account_address,
        private_key=private_key,
        max_fee=max_fee,
        gateway_url=gateway_url,
    )

    contract_address = calculate_contract_address_from_hash(
        salt=salt,
        class_hash=int(class_hash, 16),
        constructor_calldata=ctor_args,
        deployer_address=0 if not unique else int(account_address, 16),
    )
    contract_address = hex(contract_address)

    return {
        "tx_hash": invoke_tx_hash,
        "address": contract_address,
    }


def send_declare_v2(
    contract_class: ContractClass,
    compiled_class_hash: int,
    sender_address: str,
    sender_key: int,
):
    """Send a declare v2 transaction"""
    max_fee = int(1e18)  # should be enough
    version = 2
    nonce = get_nonce(sender_address)
    chain_id = StarknetChainId.TESTNET.value
    hash_value = calculate_declare_transaction_hash(
        contract_class=contract_class,
        compiled_class_hash=compiled_class_hash,
        sender_address=int(sender_address, 16),
        max_fee=max_fee,
        version=version,
        nonce=nonce,
        chain_id=chain_id,
    )

    declaration_body = Declare(
        contract_class=contract_class,
        compiled_class_hash=compiled_class_hash,
        sender_address=int(sender_address, 16),
        version=version,
        max_fee=max_fee,
        signature=list(sign(msg_hash=hash_value, priv_key=sender_key)),
        nonce=nonce,
    ).dump()
    declaration_body["type"] = "DECLARE"

    return requests.post(f"{APP_URL}/gateway/add_transaction", json=declaration_body)
