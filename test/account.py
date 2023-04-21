"""
Account test functions and utilities.
Latest changes based on https://github.com/OpenZeppelin/nile/pull/184
"""

from typing import List, Optional, Tuple

import requests
from starkware.crypto.signature.signature import private_to_stark_key, sign
from starkware.starknet.cli.starknet_cli import get_salt
from starkware.starknet.core.os.contract_address.contract_address import (
    calculate_contract_address_from_hash,
)
from starkware.starknet.core.os.transaction_hash.transaction_hash import (
    calculate_declare_transaction_hash,
    calculate_deploy_account_transaction_hash,
    calculate_deprecated_declare_transaction_hash,
)
from starkware.starknet.definitions.constants import QUERY_VERSION
from starkware.starknet.definitions.general_config import (
    DEFAULT_CHAIN_ID,
    StarknetChainId,
)
from starkware.starknet.definitions.transaction_type import TransactionType
from starkware.starknet.services.api.feeder_gateway.response_objects import (
    BlockIdentifier,
)
from starkware.starknet.services.api.gateway.transaction import (
    ContractClass,
    Declare,
    DeployAccount,
)

from starknet_devnet.account_util import AccountCall, get_execute_args
from starknet_devnet.chargeable_account import ChargeableAccount
from starknet_devnet.contract_class_wrapper import DEFAULT_ACCOUNT_HASH

from .settings import APP_URL
from .shared import EXPECTED_UDC_ADDRESS, SUPPORTED_TX_VERSION
from .util import (
    assert_hex_equal,
    estimate_fee,
    extract_class_hash,
    extract_tx_hash,
    load_contract_class,
    run_starknet,
    send_tx,
)

ACCOUNT_ARTIFACTS_PATH = "starknet_devnet/accounts_artifacts"
ACCOUNT_AUTHOR = "OpenZeppelin"
ACCOUNT_VERSION = "0.5.1"

ACCOUNT_PATH = f"{ACCOUNT_ARTIFACTS_PATH}/{ACCOUNT_AUTHOR}/{ACCOUNT_VERSION}/Account.cairo/Account.json"
ACCOUNT_ABI_PATH = f"{ACCOUNT_ARTIFACTS_PATH}/{ACCOUNT_AUTHOR}/{ACCOUNT_VERSION}/Account.cairo/Account_abi.json"

PRIVATE_KEY = 123456789987654321
PUBLIC_KEY = private_to_stark_key(PRIVATE_KEY)


def get_nonce(
    account_address: str,
    feeder_gateway_url=APP_URL,
    block_number: Optional[BlockIdentifier] = "pending",
) -> int:
    """Get nonce."""
    params = {"contractAddress": account_address}
    if block_number is not None:
        params["blockNumber"] = block_number

    resp = requests.get(
        f"{feeder_gateway_url}/feeder_gateway/get_nonce",
        params=params,
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
        nonce = get_nonce(account_address, feeder_gateway_url=feeder_gateway_url)

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
    private_key: int,
    nonce: int = None,
    max_fee: int = 0,
    gateway_url=APP_URL,
    chain_id=StarknetChainId.TESTNET,
):
    """Wrapper around starknet declare"""

    if nonce is None:
        nonce = get_nonce(account_address, feeder_gateway_url=gateway_url)

    tx_hash = calculate_deprecated_declare_transaction_hash(
        contract_class=load_contract_class(contract_path),
        chain_id=chain_id.value,
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
        ],
        gateway_url=gateway_url,
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
    chain_id=StarknetChainId.TESTNET,
):
    """Wrapper around starknet deploy"""

    # accepts ints or decimal strings
    ctor_args = [int(val) for val in inputs or []]
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
        chain_id=chain_id,
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


def declare_and_deploy(
    contract: str,
    account_address: str,
    private_key: int,
    inputs=None,
    salt=None,
    declare_max_fee=int(1e18),
    max_fee=None,
    gateway_url=APP_URL,
    chain_id=StarknetChainId.TESTNET,
):
    """
    Declare a class and deploy its instance using the provided account.
    The max_fee only refers to deployment.
    Returns deploy info with class_hash.
    """

    declare_info = declare(
        contract_path=contract,
        account_address=account_address,
        private_key=private_key,
        max_fee=declare_max_fee,
        gateway_url=gateway_url,
        chain_id=chain_id,
    )
    class_hash = declare_info["class_hash"]
    # here we could benefit from asserting the status of declaration, but it would also introduce time overhead

    deploy_info = deploy(
        class_hash=class_hash,
        account_address=account_address,
        private_key=private_key,
        inputs=inputs,
        salt=salt,
        max_fee=max_fee,
        gateway_url=gateway_url,
        chain_id=chain_id,
    )

    # expand the object with the hash of the class that was deployed
    deploy_info["class_hash"] = class_hash

    return deploy_info


def declare_and_deploy_with_chargeable(
    contract: str,
    inputs=None,
    salt=None,
    max_fee=None,
    gateway_url=APP_URL,
    chain_id=StarknetChainId.TESTNET,
):
    """
    Declare a class and deploy its instance using the chargeable account.
    The max_fee only refers to deployment.
    Returns deploy info.
    """
    return declare_and_deploy(
        contract=contract,
        account_address=hex(ChargeableAccount.ADDRESS),
        private_key=ChargeableAccount.PRIVATE_KEY,
        inputs=inputs,
        salt=salt,
        max_fee=max_fee,
        gateway_url=gateway_url,
        chain_id=chain_id,
    )


def deploy_with_chargeable(
    class_hash: str,
    inputs=None,
    salt=None,
    max_fee=None,
    gateway_url=APP_URL,
    chain_id=StarknetChainId.TESTNET,
):
    """Deploy an instance of `contract` using the chargeable account"""
    return deploy(
        class_hash=class_hash,
        account_address=hex(ChargeableAccount.ADDRESS),
        private_key=ChargeableAccount.PRIVATE_KEY,
        inputs=inputs,
        salt=salt,
        max_fee=max_fee,
        gateway_url=gateway_url,
        chain_id=chain_id,
    )


def deploy_account_contract(
    private_key: int,
    class_hash=DEFAULT_ACCOUNT_HASH,
    salt=None,
    max_fee=int(1e18),
):
    """Deploy account contract. Defaults to using a pre-created key."""

    constructor_calldata = [private_to_stark_key(private_key)]
    salt = get_salt(salt)
    account_address = calculate_contract_address_from_hash(
        salt=salt,
        class_hash=class_hash,
        constructor_calldata=constructor_calldata,
        deployer_address=0,
    )

    version = SUPPORTED_TX_VERSION
    nonce = 0
    tx_hash = calculate_deploy_account_transaction_hash(
        version=version,
        contract_address=account_address,
        class_hash=class_hash,
        constructor_calldata=constructor_calldata,
        max_fee=max_fee,
        nonce=nonce,
        salt=salt,
        chain_id=DEFAULT_CHAIN_ID,
    )

    deploy_tx = DeployAccount(
        version=version,
        max_fee=max_fee,
        signature=[int(s) for s in _get_signature(tx_hash, private_key)],
        nonce=nonce,
        class_hash=class_hash,
        contract_address_salt=salt,
        constructor_calldata=constructor_calldata,
    ).dump()

    # basically we don't need the `resp`, but when it's here, why not make assertions
    resp = send_tx(deploy_tx, TransactionType.DEPLOY_ACCOUNT)

    deploy_info = {
        "tx_hash": hex(tx_hash),
        "address": hex(account_address),
    }

    assert_hex_equal(resp["transaction_hash"], deploy_info["tx_hash"])
    assert_hex_equal(resp["address"], deploy_info["address"])

    return deploy_info


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
