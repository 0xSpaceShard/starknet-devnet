"""
Tests of v2 declaration.
"""

import pytest
import requests
from starkware.starknet.core.os.contract_class.compiled_class_hash import (
    compute_compiled_class_hash,
)
from starkware.starknet.services.api.contract_class.contract_class import (
    CompiledClass,
    ContractClass,
)
from starkware.starknet.services.api.contract_class.contract_class_utils import (
    load_sierra,
)

from .account import deploy, invoke, send_declare_v2
from .settings import APP_URL
from .shared import (
    ABI_1_PATH,
    CONTRACT_1_CASM_PATH,
    CONTRACT_1_PATH,
    PREDEPLOY_ACCOUNT_CLI_ARGS,
    PREDEPLOYED_ACCOUNT_ADDRESS,
    PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
)
from .test_state_update import get_state_update
from .util import (
    assert_hex_equal,
    assert_tx_status,
    assert_undeclared_class,
    call,
    devnet_in_background,
    get_class_by_hash,
    get_compiled_class_by_class_hash,
    get_full_contract_raw,
)


def assert_declare_v2_accepted(resp: requests.Response):
    """Assert status of declaration is accepted"""
    assert resp.status_code == 200

    declare_tx_hash = resp.json()["transaction_hash"]
    assert_tx_status(tx_hash=declare_tx_hash, expected_tx_status="ACCEPTED_ON_L2")


def _assert_already_declared(declaration_resp: requests.Response):
    assert declaration_resp.status_code == 200, declaration_resp.json()
    declare_tx_hash = declaration_resp.json()["transaction_hash"]

    tx_resp = requests.get(
        f"{APP_URL}/feeder_gateway/get_transaction",
        params={"transactionHash": declare_tx_hash},
    )
    assert tx_resp.status_code == 200, tx_resp.json()
    tx_resp_body = tx_resp.json()

    assert tx_resp_body.get("status") == "REJECTED", tx_resp_body
    assert (
        "already declared"
        in tx_resp_body["transaction_failure_reason"]["error_message"]
    )


def _assert_invalid_compiled_class_hash(declaration_resp: requests.Response):
    assert declaration_resp.status_code == 200, declaration_resp.json()
    declare_tx_hash = declaration_resp.json()["transaction_hash"]

    tx_resp = requests.get(
        f"{APP_URL}/feeder_gateway/get_transaction",
        params={"transactionHash": declare_tx_hash},
    )
    assert tx_resp.status_code == 200, tx_resp.json()
    tx_resp_body = tx_resp.json()

    assert tx_resp_body.get("status") == "REJECTED", tx_resp_body
    assert (
        "Compiled class hash not matching"
        in tx_resp_body["transaction_failure_reason"]["error_message"]
    )


def load_cairo1_contract():
    """Returns (contract_class, compiled_class, compiled_class_hash)"""
    contract_class = load_sierra(CONTRACT_1_PATH)
    with open(CONTRACT_1_CASM_PATH, encoding="utf-8") as casm_file:
        compiled_class = CompiledClass.loads(casm_file.read())
    compiled_class_hash = compute_compiled_class_hash(compiled_class)

    return contract_class, compiled_class, compiled_class_hash


@pytest.mark.declare
@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS)
def test_declare_v2_invalid_compiled_class_hash():
    """Set an invalid compiled class hash and expect failure"""
    contract_class, _, compiled_class_hash = load_cairo1_contract()
    _assert_invalid_compiled_class_hash(
        send_declare_v2(
            contract_class=contract_class,
            # invalid compiled class hash
            compiled_class_hash=compiled_class_hash + 1,
            sender_address=PREDEPLOYED_ACCOUNT_ADDRESS,
            sender_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
        )
    )


@pytest.mark.declare
@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS)
def test_redeclaring_v2():
    """Should fail if redeclaring"""
    contract_class, _, compiled_class_hash = load_cairo1_contract()
    send_declare_v2(
        contract_class=contract_class,
        compiled_class_hash=compiled_class_hash,
        sender_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        sender_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
    )

    _assert_already_declared(
        send_declare_v2(
            contract_class=contract_class,
            compiled_class_hash=compiled_class_hash,
            sender_address=PREDEPLOYED_ACCOUNT_ADDRESS,
            sender_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
        )
    )


@pytest.mark.declare
@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS)
def test_classes_available_after_declare_v2():
    """Should successfully get class and compiled class by hash"""
    # assert class present only by class hash

    contract_class, compiled_class, compiled_class_hash = load_cairo1_contract()

    declaration_resp = send_declare_v2(
        contract_class=contract_class,
        compiled_class_hash=compiled_class_hash,
        sender_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        sender_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
    )
    assert_declare_v2_accepted(declaration_resp)
    class_hash = declaration_resp.json()["class_hash"]

    assert ContractClass.load(get_class_by_hash(class_hash).json()) == contract_class
    assert_undeclared_class(get_class_by_hash(hex(compiled_class_hash)))

    # assert compiled class retrievable only by class hash
    assert (
        CompiledClass.load(get_compiled_class_by_class_hash(class_hash).json())
        == compiled_class
    )
    assert_undeclared_class(get_compiled_class_by_class_hash(hex(compiled_class_hash)))

    # assert class present in the right property of state update
    state_update = get_state_update()
    assert "state_diff" in state_update

    assert state_update["state_diff"]["old_declared_contracts"] == []
    declared_classes = state_update["state_diff"]["declared_classes"]
    assert len(declared_classes) == 1
    assert_hex_equal(declared_classes[0]["class_hash"], class_hash)
    assert_hex_equal(
        declared_classes[0]["compiled_class_hash"], hex(compiled_class_hash)
    )


def _call_get_balance(address: str) -> int:
    balance = call(
        function="get_balance",
        address=address,
        abi_path=ABI_1_PATH,
    )
    return int(balance, base=10)


@pytest.mark.declare
@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS)
def test_v2_contract_interaction():
    """Test using declare v2 and interact with contract (deploy, invoke, call)"""

    contract_class, _, compiled_class_hash = load_cairo1_contract()

    # declare
    declaration_resp = send_declare_v2(
        contract_class=contract_class,
        compiled_class_hash=compiled_class_hash,
        sender_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        sender_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
    )
    assert_declare_v2_accepted(declaration_resp)
    class_hash = declaration_resp.json()["class_hash"]

    # deploy
    initial_balance = 10
    deploy_info = deploy(
        class_hash=class_hash,
        account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
        inputs=[str(initial_balance)],
        max_fee=int(1e18),
    )
    assert_tx_status(
        tx_hash=deploy_info["tx_hash"], expected_tx_status="ACCEPTED_ON_L2"
    )

    # call after deployment
    initial_fetched_balance = _call_get_balance(deploy_info["address"])
    assert initial_fetched_balance == initial_balance

    # invoke
    increment_value = 15
    invoke_tx_hash = invoke(
        calls=[(deploy_info["address"], "increase_balance", [increment_value, 0])],
        account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
    )
    assert_tx_status(tx_hash=invoke_tx_hash, expected_tx_status="ACCEPTED_ON_L2")

    # call after invoke
    fetched_balance_after_invoke = _call_get_balance(deploy_info["address"])
    assert fetched_balance_after_invoke == initial_balance + increment_value


@pytest.mark.declare
@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS)
def test_v2_get_full_contract():
    """Test for declare, deploy and get full contract"""

    contract_class, _, compiled_class_hash = load_cairo1_contract()

    # declare
    declaration_resp = send_declare_v2(
        contract_class=contract_class,
        compiled_class_hash=compiled_class_hash,
        sender_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        sender_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
    )
    assert_declare_v2_accepted(declaration_resp)
    class_hash = declaration_resp.json()["class_hash"]

    # deploy
    initial_balance = 10
    deploy_info = deploy(
        class_hash=class_hash,
        account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
        inputs=[str(initial_balance)],
        max_fee=int(1e18),
    )
    assert_tx_status(
        tx_hash=deploy_info["tx_hash"], expected_tx_status="ACCEPTED_ON_L2"
    )

    full_contract = get_full_contract_raw(contract_address=deploy_info["address"])
    assert full_contract.status_code == 200
    sierra = ContractClass.load(full_contract.json())
    assert load_sierra(CONTRACT_1_PATH) == sierra
