"""
Account test functions and utilities.
Latest changes based on https://github.com/OpenZeppelin/nile/pull/184
"""

from typing import List, NamedTuple, Sequence, Tuple

import requests
from starkware.crypto.signature.signature import private_to_stark_key, sign
from starkware.starknet.core.os.transaction_hash.transaction_hash import (
    TransactionHashPrefix,
    calculate_declare_transaction_hash,
    calculate_transaction_hash_common,
)
from starkware.starknet.definitions.constants import QUERY_VERSION
from starkware.starknet.definitions.general_config import StarknetChainId
from starkware.starknet.public.abi import get_selector_from_name

from .settings import APP_URL
from .shared import SUPPORTED_TX_VERSION
from .util import (
    deploy,
    estimate_fee,
    extract_class_hash,
    extract_tx_hash,
    load_contract_class,
    run_starknet,
)

ACCOUNT_ARTIFACTS_PATH = "starknet_devnet/accounts_artifacts"
ACCOUNT_AUTHOR = "OpenZeppelin"
ACCOUNT_VERSION = "0.5.0"

ACCOUNT_PATH = f"{ACCOUNT_ARTIFACTS_PATH}/{ACCOUNT_AUTHOR}/{ACCOUNT_VERSION}/Account.cairo/Account.json"
ACCOUNT_ABI_PATH = f"{ACCOUNT_ARTIFACTS_PATH}/{ACCOUNT_AUTHOR}/{ACCOUNT_VERSION}/Account.cairo/Account_abi.json"

PRIVATE_KEY = 123456789987654321
PUBLIC_KEY = private_to_stark_key(PRIVATE_KEY)


def deploy_account_contract(salt=None):
    """Deploy account contract."""
    return deploy(ACCOUNT_PATH, inputs=[str(PUBLIC_KEY)], salt=salt)


def get_nonce(account_address: str, feeder_gateway_url=APP_URL) -> int:
    """Get nonce."""
    resp = requests.get(
        f"{feeder_gateway_url}/feeder_gateway/get_nonce?contractAddress={account_address}"
    )
    return int(resp.json(), 16)


def _get_execute_calldata(call_array, calldata):
    """Get calldata for __execute__."""
    return [
        len(call_array),
        *[x for t in call_array for x in t],
        len(calldata),
        *calldata,
    ]


def _get_signature(message_hash: int, private_key: int) -> Tuple[str, str]:
    """Get signature from message hash and private key."""
    sig_r, sig_s = sign(message_hash, private_key)
    return [str(sig_r), str(sig_s)]


class AccountCall(NamedTuple):
    """Things needed to interact through Account"""

    to_address: str
    """The address of the called contract"""

    function: str
    inputs: List[str]


def _from_call_to_call_array(calls: List[AccountCall]):
    """Transforms calls to call_array and calldata."""
    call_array = []
    calldata = []

    for call_tuple in calls:
        call_tuple = AccountCall(*call_tuple)

        entry = (
            int(call_tuple.to_address, 16),
            get_selector_from_name(call_tuple.function),
            len(calldata),
            len(call_tuple.inputs),
        )
        call_array.append(entry)
        calldata.extend(int(data) for data in call_tuple.inputs)

    return (call_array, calldata)


def _adapt_inputs(execute_calldata: List[int]) -> List[str]:
    """Get stringified inputs from execute_calldata."""
    return [str(v) for v in execute_calldata]


# pylint: disable=too-many-arguments
def _get_execute_args(
    calls: List[AccountCall],
    account_address: str,
    private_key: int,
    nonce: int,
    version: int,
    max_fee=None,
    chain_id=StarknetChainId.TESTNET,
):
    """Returns signature and execute calldata"""

    # get execute calldata
    (call_array, calldata) = _from_call_to_call_array(calls)
    execute_calldata = _get_execute_calldata(call_array, calldata)

    # get signature
    message_hash = _get_transaction_hash(
        contract_address=int(account_address, 16),
        calldata=execute_calldata,
        nonce=nonce,
        version=version,
        max_fee=max_fee,
        chain_id=chain_id,
    )
    signature = _get_signature(message_hash, private_key)

    return signature, execute_calldata


def _get_transaction_hash(
    contract_address: int,
    calldata: Sequence[int],
    nonce: int,
    version: int,
    max_fee: int,
    chain_id=StarknetChainId.TESTNET,
) -> str:
    """Get transaction hash for execute transaction."""
    return calculate_transaction_hash_common(
        tx_hash_prefix=TransactionHashPrefix.INVOKE,
        version=version,
        contract_address=contract_address,
        entry_point_selector=0,
        calldata=calldata,
        max_fee=max_fee,
        chain_id=chain_id.value,
        additional_data=[nonce],
    )


def get_estimated_fee(
    calls: List[AccountCall],
    account_address: str,
    private_key: str,
    nonce=None,
    feeder_gateway_url=APP_URL,
    chain_id=StarknetChainId.TESTNET,
):
    """Get estimated fee through account."""

    if nonce is None:
        nonce = get_nonce(account_address)

    signature, execute_calldata = _get_execute_args(
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
        feeder_gateway_url=feeder_gateway_url,
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

    signature, execute_calldata = _get_execute_args(
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
):
    """Wrapper around starknet declare"""

    if nonce is None:
        nonce = get_nonce(account_address)

    tx_hash = calculate_declare_transaction_hash(
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
            "--contract",
            contract_path,
            "--signature",
            *signature,
            "--sender",
            account_address,
            "--max_fee",
            str(max_fee),
        ]
    )
    return {
        "tx_hash": extract_tx_hash(output.stdout),
        "class_hash": extract_class_hash(output.stdout),
    }
