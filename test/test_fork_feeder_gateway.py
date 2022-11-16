"""Test feeder gateway responses of origin and fork"""

from .account import invoke
from .shared import (
    BALANCE_KEY,
    CONTRACT_PATH,
    EXPECTED_CLASS_HASH,
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
    assert_address_has_no_class_hash,
    assert_class_hash_at_address,
    assert_contract_code_not_present,
    assert_contract_code_present,
    assert_full_contract,
    assert_full_contract_not_present,
    assert_receipt_present,
    assert_storage,
    assert_transaction,
    assert_transaction_not_received,
    assert_transaction_receipt_not_received,
    assert_tx_status,
    deploy,
    devnet_in_background,
)


DEPLOYMENT_INPUT = "10"
EXPECTED_DEPLOYMENT_ADDRESS = (
    "0x007e723b33e317c604b36e17d0a8e7064b08eda39aa9a4d6a94a7626ec432d8a"
)
EXPECTED_INVOKE_HASH = (
    "0x51b501687e77d5433c7fc00b3a6dd25c2f6edf95f506dd9cba2251a4ce9ed43"
)


def _deploy_to_expected_address(contract=CONTRACT_PATH):
    deploy_info = deploy(
        contract=contract,
        inputs=[DEPLOYMENT_INPUT],
        salt="0x42",
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
def test_contract_responses():
    """Assert that get_full_contract only makes sense on fork after deployment"""
    # full contract
    assert_full_contract_not_present(
        address=EXPECTED_DEPLOYMENT_ADDRESS, feeder_gateway_url=TESTNET_URL
    )
    assert_full_contract_not_present(
        address=EXPECTED_DEPLOYMENT_ADDRESS, feeder_gateway_url=APP_URL
    )

    # code
    assert_contract_code_not_present(
        address=EXPECTED_DEPLOYMENT_ADDRESS, feeder_gateway_url=TESTNET_URL
    )
    assert_contract_code_not_present(
        address=EXPECTED_DEPLOYMENT_ADDRESS, feeder_gateway_url=APP_URL
    )

    # class hash
    assert_address_has_no_class_hash(
        contract_address=EXPECTED_DEPLOYMENT_ADDRESS, feeder_gateway_url=TESTNET_URL
    )
    assert_address_has_no_class_hash(
        contract_address=EXPECTED_DEPLOYMENT_ADDRESS, feeder_gateway_url=APP_URL
    )

    # storage
    assert_storage(
        address=EXPECTED_DEPLOYMENT_ADDRESS,
        key=BALANCE_KEY,
        expected_value="0x0",
        feeder_gateway_url=TESTNET_URL,
    )
    assert_storage(
        address=EXPECTED_DEPLOYMENT_ADDRESS,
        key=BALANCE_KEY,
        expected_value="0x0",
        feeder_gateway_url=APP_URL,
    )

    _deploy_to_expected_address()

    # full contract
    assert_full_contract_not_present(
        address=EXPECTED_DEPLOYMENT_ADDRESS, feeder_gateway_url=TESTNET_URL
    )
    assert_full_contract(
        address=EXPECTED_DEPLOYMENT_ADDRESS,
        expected_path=CONTRACT_PATH,
        feeder_gateway_url=APP_URL,
    )

    # code
    assert_contract_code_not_present(
        address=EXPECTED_DEPLOYMENT_ADDRESS, feeder_gateway_url=TESTNET_URL
    )
    assert_contract_code_present(
        address=EXPECTED_DEPLOYMENT_ADDRESS, feeder_gateway_url=APP_URL
    )

    assert_address_has_no_class_hash(
        contract_address=EXPECTED_DEPLOYMENT_ADDRESS, feeder_gateway_url=TESTNET_URL
    )
    assert_class_hash_at_address(
        contract_address=EXPECTED_DEPLOYMENT_ADDRESS,
        expected_class_hash=EXPECTED_CLASS_HASH,
        feeder_gateway_url=APP_URL,
    )

    # storage
    assert_storage(
        address=EXPECTED_DEPLOYMENT_ADDRESS,
        key=BALANCE_KEY,
        expected_value="0x0",
        feeder_gateway_url=TESTNET_URL,
    )
    assert_storage(
        address=EXPECTED_DEPLOYMENT_ADDRESS,
        key=BALANCE_KEY,
        expected_value=hex(int(DEPLOYMENT_INPUT, 10)),
        feeder_gateway_url=APP_URL,
    )


def test_get_class_by_hash():
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


def test_estimate_fee():
    raise NotImplementedError


def test_simulate_transaction():
    raise NotImplementedError


def test_block_responses():
    def test_get_block():
        raise NotImplementedError

    def test_get_block_traces():
        raise NotImplementedError

    def test_get_state_update():
        raise NotImplementedError
