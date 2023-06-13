"""
Tests RPC traces
"""

from __future__ import annotations

import copy
from test.account import _get_signature, get_nonce
from test.rpc.conftest import prepare_deploy_account_tx, rpc_deploy_account_from_gateway
from test.rpc.rpc_utils import (
    get_predeployed_acc_execute_args,
    rpc_call_background_devnet,
)
from test.rpc.test_rpc_transactions import pad_zero_entry_points
from test.shared import (
    PREDEPLOY_ACCOUNT_CLI_ARGS,
    PREDEPLOYED_ACCOUNT_ADDRESS,
    PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
    SUPPORTED_RPC_TX_VERSION,
)
from test.test_account import deploy_empty_contract
from test.test_declare_v2 import load_cairo1_contract

from starkware.starknet.core.os.transaction_hash.transaction_hash import (
    calculate_declare_transaction_hash,
    calculate_deprecated_declare_transaction_hash,
)
from starkware.starknet.definitions.general_config import StarknetChainId
from starkware.starknet.services.api.contract_class.contract_class import (
    DeprecatedCompiledClass,
)
from starkware.starknet.services.api.gateway.transaction_utils import decompress_program

from starknet_devnet.blueprints.rpc.structures.payloads import (
    RpcBroadcastedDeclareTxnV1,
    RpcBroadcastedDeclareTxnV2,
    RpcBroadcastedInvokeTxnV1,
    RpcDeprecatedContractClass,
    SimulationFlag,
    rpc_contract_class,
)
from starknet_devnet.blueprints.rpc.utils import rpc_felt
from starknet_devnet.constants import (
    DEPRECATED_RPC_DECLARE_TX_VERSION,
    SUPPORTED_RPC_DECLARE_TX_VERSION,
)

from ..util import devnet_in_background


@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS)
def test_skip_execute_flag(deploy_account_details):
    """Test if simulate_transaction with SKIP_EXECUTE flag is raising an exception."""
    deploy_account_tx, _ = prepare_deploy_account_tx(**deploy_account_details)
    rpc_deploy_account_tx = rpc_deploy_account_from_gateway(deploy_account_tx)

    response = rpc_call_background_devnet(
        "starknet_simulateTransaction",
        {
            "block_id": "latest",
            "transaction": [rpc_deploy_account_tx],
            "simulation_flags": [SimulationFlag.SKIP_EXECUTE.name],
        },
    )
    assert response["error"]["message"] == "SKIP_EXECUTE flag is not supported"


@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS)
def test_simulate_transaction_invoke():
    """Test simulate_transaction with invoke transaction"""
    contract_address = deploy_empty_contract()["address"]

    calls = [(contract_address, "sum_point_array", [2, 10, 20, 30, 40])]
    signature, execute_calldata = get_predeployed_acc_execute_args(calls)

    invoke_transaction = RpcBroadcastedInvokeTxnV1(
        type="INVOKE",
        max_fee=rpc_felt(0),
        version=hex(SUPPORTED_RPC_TX_VERSION),
        signature=[rpc_felt(sig) for sig in signature],
        nonce=rpc_felt(get_nonce(PREDEPLOYED_ACCOUNT_ADDRESS)),
        sender_address=rpc_felt(PREDEPLOYED_ACCOUNT_ADDRESS),
        calldata=[rpc_felt(data) for data in execute_calldata],
    )

    response_no_flags = rpc_call_background_devnet(
        "starknet_simulateTransaction",
        {
            "block_id": "latest",
            "transaction": [invoke_transaction],
            "simulation_flags": [],
        },
    )["result"][0]
    response_skip_flag = rpc_call_background_devnet(
        "starknet_simulateTransaction",
        {
            "block_id": "latest",
            "transaction": [invoke_transaction],
            "simulation_flags": [SimulationFlag.SKIP_VALIDATE.name],
        },
    )["result"][0]

    assert int(response_no_flags["fee_estimation"][0]["overall_fee"], 0) > int(
        response_skip_flag["fee_estimation"][0]["overall_fee"], 0
    )
    assert response_no_flags["transaction_trace"][0]["validate_invocation"][
        "contract_address"
    ] == rpc_felt(PREDEPLOYED_ACCOUNT_ADDRESS)
    assert response_skip_flag["transaction_trace"][0]["validate_invocation"] is None
    assert response_no_flags["transaction_trace"][0]["execute_invocation"][
        "contract_address"
    ] == rpc_felt(PREDEPLOYED_ACCOUNT_ADDRESS)
    assert response_skip_flag["transaction_trace"][0]["execute_invocation"][
        "contract_address"
    ] == rpc_felt(PREDEPLOYED_ACCOUNT_ADDRESS)
    assert response_no_flags["transaction_trace"][0]["fee_transfer_invocation"] is None
    assert response_skip_flag["transaction_trace"][0]["fee_transfer_invocation"] is None


@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS)
def test_simulate_transaction_declare_v1(declare_content):
    """Test simulate_transaction with declare v1 transaction"""
    contract_class = declare_content["contract_class"]
    pad_zero_entry_points(contract_class["entry_points_by_type"])

    _rpc_contract_class = RpcDeprecatedContractClass(
        program=contract_class["program"],
        entry_points_by_type=copy.deepcopy(contract_class["entry_points_by_type"]),
        abi=contract_class["abi"],
    )

    contract_class["program"] = decompress_program(contract_class["program"])
    contract_class = DeprecatedCompiledClass.load(contract_class)

    nonce = get_nonce(PREDEPLOYED_ACCOUNT_ADDRESS)
    tx_hash = calculate_deprecated_declare_transaction_hash(
        contract_class=contract_class,
        chain_id=StarknetChainId.TESTNET.value,
        sender_address=int(PREDEPLOYED_ACCOUNT_ADDRESS, 16),
        max_fee=0,
        nonce=nonce,
        version=DEPRECATED_RPC_DECLARE_TX_VERSION,
    )
    signature = _get_signature(tx_hash, PREDEPLOYED_ACCOUNT_PRIVATE_KEY)

    declare_transaction = RpcBroadcastedDeclareTxnV1(
        type=declare_content["type"],
        max_fee=rpc_felt(0),
        version=hex(DEPRECATED_RPC_DECLARE_TX_VERSION),
        signature=[rpc_felt(sig) for sig in signature],
        nonce=rpc_felt(nonce),
        contract_class=_rpc_contract_class,
        sender_address=rpc_felt(PREDEPLOYED_ACCOUNT_ADDRESS),
    )

    response_no_flags = rpc_call_background_devnet(
        "starknet_simulateTransaction",
        {
            "block_id": "latest",
            "transaction": [declare_transaction],
            "simulation_flags": [],
        },
    )["result"][0]
    response_skip_flag = rpc_call_background_devnet(
        "starknet_simulateTransaction",
        {
            "block_id": "latest",
            "transaction": [declare_transaction],
            "simulation_flags": [SimulationFlag.SKIP_VALIDATE.name],
        },
    )["result"][0]

    assert int(response_no_flags["fee_estimation"][0]["overall_fee"], 0) > int(
        response_skip_flag["fee_estimation"][0]["overall_fee"], 0
    )
    assert response_no_flags["transaction_trace"][0]["validate_invocation"][
        "contract_address"
    ] == rpc_felt(PREDEPLOYED_ACCOUNT_ADDRESS)
    assert response_skip_flag["transaction_trace"][0]["validate_invocation"] is None
    assert response_no_flags["transaction_trace"][0]["fee_transfer_invocation"] is None
    assert response_skip_flag["transaction_trace"][0]["fee_transfer_invocation"] is None


@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS)
def test_simulate_transaction_declare_v2():
    """Test simulate_transaction with declare v2 transaction"""
    contract_class, _, compiled_class_hash = load_cairo1_contract()

    nonce = get_nonce(PREDEPLOYED_ACCOUNT_ADDRESS)

    tx_hash = calculate_declare_transaction_hash(
        contract_class=contract_class,
        compiled_class_hash=compiled_class_hash,
        chain_id=StarknetChainId.TESTNET.value,
        sender_address=int(PREDEPLOYED_ACCOUNT_ADDRESS, 16),
        max_fee=0,
        version=SUPPORTED_RPC_DECLARE_TX_VERSION,
        nonce=nonce,
    )

    signature = _get_signature(tx_hash, PREDEPLOYED_ACCOUNT_PRIVATE_KEY)

    declare_transaction = RpcBroadcastedDeclareTxnV2(
        contract_class=rpc_contract_class(contract_class),
        sender_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        compiled_class_hash=rpc_felt(compiled_class_hash),
        type="DECLARE",
        version=rpc_felt(SUPPORTED_RPC_DECLARE_TX_VERSION),
        nonce=rpc_felt(nonce),
        max_fee=rpc_felt(0),
        signature=list(map(rpc_felt, signature)),
    )

    response_no_flags = rpc_call_background_devnet(
        "starknet_simulateTransaction",
        {
            "block_id": "latest",
            "transaction": [declare_transaction],
            "simulation_flags": [],
        },
    )["result"][0]
    response_skip_flag = rpc_call_background_devnet(
        "starknet_simulateTransaction",
        {
            "block_id": "latest",
            "transaction": [declare_transaction],
            "simulation_flags": [SimulationFlag.SKIP_VALIDATE.name],
        },
    )["result"][0]

    assert int(response_no_flags["fee_estimation"][0]["overall_fee"], 0) > int(
        response_skip_flag["fee_estimation"][0]["overall_fee"], 0
    )
    assert response_no_flags["transaction_trace"][0]["validate_invocation"][
        "contract_address"
    ] == rpc_felt(PREDEPLOYED_ACCOUNT_ADDRESS)
    assert response_skip_flag["transaction_trace"][0]["validate_invocation"] is None
    assert response_no_flags["transaction_trace"][0]["fee_transfer_invocation"] is None
    assert response_skip_flag["transaction_trace"][0]["fee_transfer_invocation"] is None


@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS)
def test_simulate_transaction_deploy_account(deploy_account_details):
    """Test simulate_transaction with deploy account transaction"""
    deploy_account_tx, deploy_account_contract_address = prepare_deploy_account_tx(
        **deploy_account_details
    )
    rpc_deploy_account_tx = rpc_deploy_account_from_gateway(deploy_account_tx)

    response_no_flags = rpc_call_background_devnet(
        "starknet_simulateTransaction",
        {
            "block_id": "latest",
            "transaction": [rpc_deploy_account_tx],
            "simulation_flags": [],
        },
    )["result"][0]
    response_skip_flag = rpc_call_background_devnet(
        "starknet_simulateTransaction",
        {
            "block_id": "latest",
            "transaction": [rpc_deploy_account_tx],
            "simulation_flags": [SimulationFlag.SKIP_VALIDATE.name],
        },
    )["result"][0]

    assert int(response_no_flags["fee_estimation"][0]["overall_fee"], 0) > int(
        response_skip_flag["fee_estimation"][0]["overall_fee"], 0
    )
    assert response_no_flags["transaction_trace"][0]["validate_invocation"][
        "contract_address"
    ] == rpc_felt(deploy_account_contract_address)
    assert response_skip_flag["transaction_trace"][0]["validate_invocation"] is None
    assert response_no_flags["transaction_trace"][0]["fee_transfer_invocation"] is None
    assert response_no_flags["transaction_trace"][0]["constructor_invocation"][
        "contract_address"
    ] == rpc_felt(deploy_account_contract_address)
    assert response_skip_flag["transaction_trace"][0]["fee_transfer_invocation"] is None
    assert response_skip_flag["transaction_trace"][0]["constructor_invocation"][
        "contract_address"
    ] == rpc_felt(deploy_account_contract_address)
