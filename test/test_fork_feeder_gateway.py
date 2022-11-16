"""Test feeder gateway responses of origin and fork"""

from .account import invoke
from .shared import (
    CONTRACT_PATH,
    PREDEPLOYED_ACCOUNT_ADDRESS,
    PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
)
from .settings import APP_URL
from .test_transaction_trace import get_transaction_trace_response
from .testnet_deployment import (
    TESTNET_CONTRACT_ADDRESS,
    TESTNET_DEPLOYMENT_BLOCK,
    TESTNET_FORK_PARAMS,
    TESTNET_URL,
)
from .util import (
    assert_contract_code_not_present,
    assert_contract_code_present,
    assert_receipt_present,
    assert_transaction,
    assert_transaction_not_received,
    assert_transaction_receipt_not_received,
    assert_tx_status,
    deploy,
    devnet_in_background,
)


def test_get_block():
    raise NotImplementedError


def test_get_block_traces():
    raise NotImplementedError


EXPECTED_DEPLOYMENT_ADDRESS = (
    "0x007e723b33e317c604b36e17d0a8e7064b08eda39aa9a4d6a94a7626ec432d8a"
)
EXPECTED_CLASS_HASH = (
    "0x028c7d54caa154d29953a26857c200623fd185bffa178a185d0ff247d22127a9"
)
EXPECTED_INVOKE_HASH = (
    "0x51b501687e77d5433c7fc00b3a6dd25c2f6edf95f506dd9cba2251a4ce9ed43"
)


def _deploy_to_expected_address(contract=CONTRACT_PATH, inputs=("10",), salt="0x42"):
    deploy_info = deploy(
        contract=contract,
        inputs=inputs,
        salt=salt,
    )
    assert int(deploy_info["address"], 16) == int(EXPECTED_DEPLOYMENT_ADDRESS, 16)


def _make_expected_invoke(gateway_url=APP_URL):
    invoke_tx_hash = invoke(
        calls=[(TESTNET_CONTRACT_ADDRESS, "increase_balance", [1, 2])],
        account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
        gateway_url=gateway_url,
    )
    assert int(invoke_tx_hash, 16) == int(EXPECTED_INVOKE_HASH, 16)


@devnet_in_background(
    *TESTNET_FORK_PARAMS,
    "--fork-block",
    # starting from an earlier block; otherwise the contract class is already present
    str(TESTNET_DEPLOYMENT_BLOCK - 1),
)
def test_get_code():
    """Assert that get_code only makes sense on fork after deployment"""

    assert_contract_code_not_present(
        address=EXPECTED_DEPLOYMENT_ADDRESS, feeder_gateway_url=TESTNET_URL
    )
    assert_contract_code_not_present(
        address=EXPECTED_DEPLOYMENT_ADDRESS, feeder_gateway_url=APP_URL
    )

    _deploy_to_expected_address()

    assert_contract_code_not_present(
        address=EXPECTED_DEPLOYMENT_ADDRESS, feeder_gateway_url=TESTNET_URL
    )
    assert_contract_code_present(
        address=EXPECTED_DEPLOYMENT_ADDRESS, feeder_gateway_url=APP_URL
    )


@devnet_in_background(
    *TESTNET_FORK_PARAMS,
    "--fork-block",
    # starting from an earlier block; otherwise the contract class is already present
    str(TESTNET_DEPLOYMENT_BLOCK - 1),
)
def test_get_full_contract():
    """Assert that get_full_contract only makes sense on fork after deployment"""
    assert_contract_code_not_present(
        address=EXPECTED_DEPLOYMENT_ADDRESS, feeder_gateway_url=TESTNET_URL
    )
    assert_contract_code_not_present(
        address=EXPECTED_DEPLOYMENT_ADDRESS, feeder_gateway_url=APP_URL
    )

    _deploy_to_expected_address()

    assert_contract_code_not_present(
        address=EXPECTED_DEPLOYMENT_ADDRESS, feeder_gateway_url=TESTNET_URL
    )
    assert_contract_code_present(
        address=EXPECTED_DEPLOYMENT_ADDRESS, feeder_gateway_url=APP_URL
    )


def test_get_class_hash_at():
    raise NotImplementedError


def test_get_class_by_hash():
    raise NotImplementedError


def test_get_storage_at():
    raise NotImplementedError


def _assert_transaction_trace_not_present(tx_hash: str, feeder_gateway_url=APP_URL):
    resp = get_transaction_trace_response(tx_hash, server_url=feeder_gateway_url)
    assert resp.status_code == 500


def _assert_transaction_trace_present(
    tx_hash: str, expected_address: str, feeder_gateway_url=APP_URL
):
    resp = get_transaction_trace_response(tx_hash, server_url=feeder_gateway_url)
    body = resp.json()
    assert body["function_invocation"]["contract_address"] == expected_address
    assert resp.status_code == 200


@devnet_in_background(
    *TESTNET_FORK_PARAMS, "--fork-block", str(TESTNET_DEPLOYMENT_BLOCK)
)
def test_transaction_responses():
    """Assert transaction only present on fork after invoking"""

    # tx status
    assert_tx_status(
        tx_hash=EXPECTED_INVOKE_HASH,
        expected_tx_status="NOT_RECEIVED",
        feeder_gateway_url=TESTNET_URL,
    )
    assert_tx_status(
        tx_hash=EXPECTED_INVOKE_HASH,
        expected_tx_status="NOT_RECEIVED",
        feeder_gateway_url=APP_URL,
    )

    # tx
    assert_transaction_not_received(
        tx_hash=EXPECTED_INVOKE_HASH, feeder_gateway_url=TESTNET_URL
    )
    assert_transaction_not_received(
        tx_hash=EXPECTED_INVOKE_HASH, feeder_gateway_url=APP_URL
    )

    # tx receipt
    assert_transaction_receipt_not_received(
        tx_hash=EXPECTED_INVOKE_HASH, feeder_gateway_url=TESTNET_URL
    )
    assert_transaction_receipt_not_received(
        tx_hash=EXPECTED_INVOKE_HASH, feeder_gateway_url=APP_URL
    )

    # tx trace
    _assert_transaction_trace_not_present(
        tx_hash=EXPECTED_INVOKE_HASH, feeder_gateway_url=TESTNET_URL
    )
    _assert_transaction_trace_not_present(
        tx_hash=EXPECTED_INVOKE_HASH, feeder_gateway_url=APP_URL
    )

    _make_expected_invoke()

    # tx status
    assert_tx_status(
        tx_hash=EXPECTED_INVOKE_HASH,
        expected_tx_status="NOT_RECEIVED",
        feeder_gateway_url=TESTNET_URL,
    )
    assert_tx_status(
        tx_hash=EXPECTED_INVOKE_HASH,
        expected_tx_status="ACCEPTED_ON_L2",
        feeder_gateway_url=APP_URL,
    )

    # tx
    assert_transaction_not_received(
        tx_hash=EXPECTED_INVOKE_HASH, feeder_gateway_url=TESTNET_URL
    )
    assert_transaction(
        tx_hash=EXPECTED_INVOKE_HASH,
        expected_status="ACCEPTED_ON_L2",
        feeder_gateway_url=APP_URL,
    )

    # tx receipt
    assert_transaction_receipt_not_received(
        tx_hash=EXPECTED_INVOKE_HASH, feeder_gateway_url=TESTNET_URL
    )
    assert_receipt_present(
        tx_hash=EXPECTED_INVOKE_HASH,
        expected_status="ACCEPTED_ON_L2",
        feeder_gateway_url=APP_URL,
    )

    # tx trace
    _assert_transaction_trace_not_present(
        tx_hash=EXPECTED_INVOKE_HASH, feeder_gateway_url=TESTNET_URL
    )
    _assert_transaction_trace_present(
        tx_hash=EXPECTED_INVOKE_HASH,
        expected_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        feeder_gateway_url=APP_URL,
    )


def test_get_state_update():
    raise NotImplementedError


def test_estimate_fee():
    raise NotImplementedError


def test_simulate_transaction():
    raise NotImplementedError


def test_estimate_message_fee():
    raise NotImplementedError
