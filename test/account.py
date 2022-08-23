"""
Account test functions and utilities.
"""

from typing import List, Sequence, Tuple

from starkware.crypto.signature.signature import private_to_stark_key, sign
from starkware.starknet.public.abi import get_selector_from_name
from starkware.starknet.definitions.constants import TRANSACTION_VERSION, QUERY_VERSION
from starkware.starknet.core.os.transaction_hash.transaction_hash import (
    calculate_transaction_hash_common,
    TransactionHashPrefix
)
from starkware.starknet.definitions.general_config import StarknetChainId

from .util import deploy, call, invoke, estimate_fee

ACCOUNT_ARTIFACTS_PATH = "starknet_devnet/accounts_artifacts"
ACCOUNT_AUTHOR = "OpenZeppelin"
ACCOUNT_VERSION = "0.3.1"

ACCOUNT_PATH = f"{ACCOUNT_ARTIFACTS_PATH}/{ACCOUNT_AUTHOR}/{ACCOUNT_VERSION}/Account.cairo/Account.json"
ACCOUNT_ABI_PATH = f"{ACCOUNT_ARTIFACTS_PATH}/{ACCOUNT_AUTHOR}/{ACCOUNT_VERSION}/Account.cairo/Account_abi.json"

PRIVATE_KEY = 123456789987654321
PUBLIC_KEY = private_to_stark_key(PRIVATE_KEY)

def deploy_account_contract(salt=None):
    """Deploy account contract."""
    return deploy(ACCOUNT_PATH, inputs=[str(PUBLIC_KEY)], salt=salt)

def get_nonce(account_address):
    """Get nonce."""
    return call("get_nonce", account_address, ACCOUNT_ABI_PATH)

def get_execute_calldata(call_array, calldata, nonce):
    """Get calldata for __execute__."""
    return [
        len(call_array),
        *[x for t in call_array for x in t],
        len(calldata),
        *calldata,
        int(nonce)
    ]

def str_to_felt(text: str) -> int:
    """Converts string to felt."""
    return int.from_bytes(bytes(text, "ascii"), "big")

def get_signature(message_hash: int, private_key: int) -> Tuple[str, str]:
    """Get signature from message hash and private key."""
    sig_r, sig_s = sign(message_hash, private_key)
    return [str(sig_r), str(sig_s)]

def from_call_to_call_array(calls):
    """Transforms calls to call_array and calldata."""
    call_array = []
    calldata = []

    for call_tuple in calls:
        assert len(call_tuple) == 3, "Invalid call parameters"

        entry = (
            call_tuple[0],
            get_selector_from_name(call_tuple[1]),
            len(calldata),
            len(call_tuple[2])
        )
        call_array.append(entry)
        calldata.extend(call_tuple[2])

    return (call_array, calldata)

def adapt_inputs(execute_calldata: List[int]) -> List[str]:
    """Get stringified inputs from execute_calldata."""
    return [str(v) for v in execute_calldata]

# pylint: disable=too-many-arguments
def get_execute_args(
    calls,
    account_address,
    private_key,
    nonce=None,
    max_fee=0,
    version: int = TRANSACTION_VERSION):
    """Returns signature and execute calldata"""

    if nonce is None:
        nonce = get_nonce(account_address)

    # get execute calldata
    (call_array, calldata) = from_call_to_call_array(calls)
    execute_calldata = get_execute_calldata(call_array, calldata, nonce)

    # get signature
    message_hash = get_transaction_hash(
        contract_address=int(account_address, 16),
        calldata=execute_calldata,
        version=version,
        max_fee=max_fee
    )
    signature = get_signature(message_hash, private_key)

    return signature, execute_calldata

def get_transaction_hash(
    contract_address: int,
    calldata: Sequence[int],
    version: int,
    max_fee: int = 0
) -> str:
    """Get transaction hash for execute transaction."""
    return calculate_transaction_hash_common(
        tx_hash_prefix=TransactionHashPrefix.INVOKE,
        version=version,
        contract_address=contract_address,
        entry_point_selector=get_selector_from_name("__execute__"),
        calldata=calldata,
        max_fee=max_fee,
        chain_id=StarknetChainId.TESTNET.value,
        additional_data=[],
    )

def get_estimated_fee(calls, account_address, private_key, nonce=None):
    """Get estimated fee through account."""
    signature, execute_calldata = get_execute_args(
        calls=calls,
        account_address=account_address,
        private_key=private_key,
        nonce=nonce,
        version=QUERY_VERSION
    )

    return estimate_fee(
        "__execute__",
        inputs=adapt_inputs(execute_calldata),
        address=account_address,
        abi_path=ACCOUNT_ABI_PATH,
        signature=signature,
    )


def execute(calls, account_address, private_key, nonce=None, max_fee=0, query=False):
    """Invoke __execute__ with correct calldata and signature."""
    if query:
        version = QUERY_VERSION
        runner = call
    else:
        version = TRANSACTION_VERSION
        runner = invoke

    signature, execute_calldata = get_execute_args(calls, account_address, private_key, nonce, max_fee, version=version)

    return runner(
        "__execute__",
        inputs=adapt_inputs(execute_calldata),
        address=account_address,
        abi_path=ACCOUNT_ABI_PATH,
        signature=signature,
        max_fee=str(max_fee)
    )
