"""
Tests RPC traces
"""

from __future__ import annotations

import copy
from test.account import _get_signature, get_nonce
from test.rpc.rpc_utils import rpc_call_background_devnet
from test.rpc.test_rpc_transactions import pad_zero_entry_points
from test.shared import (
    PREDEPLOY_ACCOUNT_CLI_ARGS,
    PREDEPLOYED_ACCOUNT_ADDRESS,
    PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
    SUPPORTED_RPC_TX_VERSION,
)
from test.test_account import deploy_empty_contract
from test.test_declare_v2 import load_cairo1_contract
from test.util import devnet_in_background

import pytest
from starkware.starknet.core.os.transaction_hash.transaction_hash import (
    calculate_declare_transaction_hash,
    calculate_deprecated_declare_transaction_hash,
)
from starkware.starknet.definitions.general_config import StarknetChainId
from starkware.starknet.public.abi import get_selector_from_name
from starkware.starknet.services.api.contract_class.contract_class import (
    DeprecatedCompiledClass,
)
from starkware.starknet.services.api.gateway.transaction_utils import decompress_program

from starknet_devnet.account_util import get_execute_args
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


# move to common file
def get_predeployed_acc_execute_args(calls):
    """Get execute arguments with predeployed account"""
    return get_execute_args(
        calls=calls,
        account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
        nonce=0,
        version=SUPPORTED_RPC_TX_VERSION,
        max_fee=0,
    )


@pytest.mark.usefixtures("run_devnet_in_background")
@pytest.mark.parametrize(
    "run_devnet_in_background, simulation_flags",
    [
        (
            [*PREDEPLOY_ACCOUNT_CLI_ARGS],
            [],
        ),
        ([*PREDEPLOY_ACCOUNT_CLI_ARGS], [SimulationFlag.SKIP_VALIDATE.name]),
    ],
    indirect=["run_devnet_in_background"],
)
def test_simulate_transaction_invoke(simulation_flags):
    """Happy path for simulate_transaction call with invoke transaction"""
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

    response = rpc_call_background_devnet(
        "starknet_simulateTransaction",
        {
            "block_id": "latest",
            "transaction": [invoke_transaction],
            "simulation_flags": simulation_flags,
        },
    )

    if not simulation_flags:
        assert (
            response["result"][0]["fee_estimation"][0]["overall_fee"] == "0x1d91ca3600"
        )
        assert response["result"][0]["transaction_trace"][0]["validate_invocation"][
            "contract_address"
        ] == rpc_felt(PREDEPLOYED_ACCOUNT_ADDRESS)
    else:
        assert (
            response["result"][0]["fee_estimation"][0]["overall_fee"] == "0x1d85de7400"
        )
        assert (
            response["result"][0]["transaction_trace"][0]["validate_invocation"] == None
        )

    assert response["result"][0]["transaction_trace"][0]["execute_invocation"][
        "contract_address"
    ] == rpc_felt(PREDEPLOYED_ACCOUNT_ADDRESS)
    assert (
        response["result"][0]["transaction_trace"][0]["fee_transfer_invocation"] == None
    )


@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS)
def test_simulate_transaction_declare_v1(declare_content):
    """Test simulate_transaction with declare transaction"""
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

    response = rpc_call_background_devnet(
        "starknet_simulateTransaction",
        {
            "block_id": "latest",
            "transaction": [declare_transaction],
            "simulation_flags": [],
        },
    )

    assert response["result"][0]["fee_estimation"][0]["overall_fee"] == "0x1d2c764500"
    assert (
        response["result"][0]["transaction_trace"][0]["fee_transfer_invocation"] == None
    )
    assert response["result"][0]["transaction_trace"][0]["validate_invocation"][
        "contract_address"
    ] == rpc_felt(PREDEPLOYED_ACCOUNT_ADDRESS)
