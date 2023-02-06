"""
Utilities for OZ (not Starknet CLI) implementation of Starknet Account
Latest changes based on https://github.com/OpenZeppelin/nile/pull/184
"""

from typing import List, NamedTuple, Sequence, Tuple

from starkware.cairo.lang.vm.crypto import pedersen_hash
from starkware.crypto.signature.signature import sign
from starkware.starknet.core.os.transaction_hash.transaction_hash import (
    TransactionHashPrefix,
    calculate_transaction_hash_common,
)
from starkware.starknet.definitions.general_config import StarknetChainId
from starkware.starknet.public.abi import get_selector_from_name
from starkware.starknet.testing.starknet import StarknetState

from .util import Uint256


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


def _get_execute_calldata(call_array, calldata):
    """Get calldata for __execute__."""
    return [
        len(call_array),
        *[x for t in call_array for x in t],
        len(calldata),
        *calldata,
    ]


# pylint: disable=too-many-arguments
def get_execute_args(
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


def _get_signature(message_hash: int, private_key: int) -> Tuple[str, str]:
    """Get signature from message hash and private key."""
    sig_r, sig_s = sign(message_hash, private_key)
    return [str(sig_r), str(sig_s)]


async def set_balance(state: StarknetState, address: int, balance: int):
    """Modify `state` so that `address` has `balance`"""

    fee_token_address = state.general_config.fee_token_address

    balance_address = pedersen_hash(get_selector_from_name("ERC20_balances"), address)
    balance_uint256 = Uint256.from_felt(balance)

    await state.state.set_storage_at(
        fee_token_address, balance_address, balance_uint256.low
    )
    await state.state.set_storage_at(
        fee_token_address, balance_address + 1, balance_uint256.high
    )
