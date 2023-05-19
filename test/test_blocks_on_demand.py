"""
Test blocks on demand mode.
"""

import requests
from starkware.starknet.definitions.error_codes import StarknetErrorCode

from starknet_devnet.blueprints.rpc.utils import rpc_felt

from .account import declare_and_deploy_with_chargeable, get_estimated_fee, invoke
from .rpc.rpc_utils import rpc_call
from .settings import APP_URL
from .shared import (
    ABI_PATH,
    CONTRACT_PATH,
    EVENTS_CONTRACT_PATH,
    INCREASE_BALANCE_CALLED_EVENT_KEY,
    PREDEPLOY_ACCOUNT_CLI_ARGS,
    PREDEPLOYED_ACCOUNT_ADDRESS,
    PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
)
from .test_state_update import get_state_update
from .test_transaction_trace import get_block_traces
from .util import (
    ErrorExpector,
    assert_equal,
    assert_hex_equal,
    assert_tx_status,
    call,
    demand_block_creation,
    devnet_in_background,
    get_block,
    increase_time,
    set_time,
)


def _get_block_resp(block_number):
    return requests.get(
        f"{APP_URL}/feeder_gateway/get_block", {"blockNumber": block_number}
    )


def _assert_block_is_pending(block: dict):
    assert block["status"] == "PENDING"
    for prop in ["block_hash", "block_number", "state_root"]:
        assert prop not in block


@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS, "--blocks-on-demand")
def test_invokable_on_pending_block():
    """Test deploy+invoke+call in blocks-on-demand mode"""
    latest_block = get_block(block_number="latest", parse=True)
    genesis_block_number = latest_block["block_number"]
    assert genesis_block_number == 0

    deploy_info = declare_and_deploy_with_chargeable(CONTRACT_PATH, inputs=["0"])
    assert_tx_status(deploy_info["tx_hash"], "PENDING")

    def get_contract_balance():
        return call(
            function="get_balance",
            address=deploy_info["address"],
            abi_path=ABI_PATH,
            block_number="latest",
        )

    with ErrorExpector(StarknetErrorCode.UNINITIALIZED_CONTRACT):
        get_contract_balance()

    invoke_hash = invoke(
        calls=[(deploy_info["address"], "increase_balance", [10, 20])],
        account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
    )
    assert_tx_status(invoke_hash, "PENDING")

    latest_block = get_block(block_number="latest", parse=True)
    block_number_after_deploy_and_invoke = latest_block["block_number"]
    assert block_number_after_deploy_and_invoke == 0

    demand_block_creation()
    assert_tx_status(invoke_hash, "ACCEPTED_ON_L2")

    balance_after_create_block = get_contract_balance()
    assert int(balance_after_create_block) == 30

    latest_block = get_block(block_number="latest", parse=True)
    block_number_after_block_on_demand_call = latest_block["block_number"]
    assert block_number_after_block_on_demand_call == 1
    assert len(latest_block["transactions"]) == 3  # declare + deploy + invoke


@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS, "--blocks-on-demand")
def test_estimation_works_after_block_creation():
    """Test estimation works only after demanding block creation."""
    deploy_info = declare_and_deploy_with_chargeable(CONTRACT_PATH, inputs=["0"])
    assert_tx_status(deploy_info["tx_hash"], "PENDING")

    def estimate_invoke_fee():
        return get_estimated_fee(
            calls=[(deploy_info["address"], "increase_balance", [10, 20])],
            account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
            private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
            block_number="latest",
        )

    with ErrorExpector(StarknetErrorCode.UNINITIALIZED_CONTRACT):
        estimate_invoke_fee()

    demand_block_creation()
    assert_tx_status(deploy_info["tx_hash"], "ACCEPTED_ON_L2")
    estimated_fee = estimate_invoke_fee()
    assert estimated_fee > 0


@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS, "--blocks-on-demand")
def test_calling_works_after_block_creation():
    """
    Test deploy in blocks-on-demand mode for invoke and contract call.
    Balance after invoke should be 0 even when we increased it.
    Only after calling create_block balance should be increased in this mode.
    """
    # Deploy and invoke
    deploy_info = declare_and_deploy_with_chargeable(CONTRACT_PATH, inputs=["0"])
    demand_block_creation()

    def get_contract_balance():
        return call(
            function="get_balance",
            address=deploy_info["address"],
            abi_path=ABI_PATH,
            block_number="latest",
        )

    balance_after_deploy = get_contract_balance()
    assert int(balance_after_deploy) == 0

    invoke(
        calls=[(deploy_info["address"], "increase_balance", [10, 20])],
        account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
    )
    balance_after_invoke = get_contract_balance()
    assert int(balance_after_invoke) == 0

    demand_block_creation()
    balance_after_create_block = get_contract_balance()
    assert int(balance_after_create_block) == 30


@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS, "--blocks-on-demand")
def test_getting_next_block():
    """Test that artifacts related to block 1 are available only after creating on demand"""

    # some transaction, could be anything
    declare_and_deploy_with_chargeable(CONTRACT_PATH, inputs=["0"])

    # expect failure on block retrieval
    next_block_number = 1
    block_resp = _get_block_resp(block_number=next_block_number)
    assert block_resp.status_code == 500
    assert block_resp.json()["code"] == str(StarknetErrorCode.BLOCK_NOT_FOUND)

    demand_block_creation()

    # expect success on block retrieval
    block_resp = _get_block_resp(block_number=next_block_number)
    assert block_resp.status_code == 200


@devnet_in_background("--blocks-on-demand")
def test_getting_pending_defaults_to_latest():
    """Test that specifying "pending" defaults to using "latest" if no there is no pending block"""

    pending_block = get_block(block_number="pending", parse=True)
    latest_block = get_block(block_number="latest", parse=True)
    assert_equal(pending_block, latest_block)

    pending_block_traces = get_block_traces({"blockNumber": "pending"})
    latest_block_traces = get_block_traces({"blockNumber": "latest"})
    assert_equal(pending_block_traces, latest_block_traces)

    pending_state_update = get_state_update(block_number="pending")
    latest_state_update = get_state_update(block_number="latest")
    assert_equal(pending_state_update, latest_state_update)


@devnet_in_background("--blocks-on-demand")
def test_pending_block():
    """Test that pending block contains pending data"""

    # get state of latest before the tx
    latest_block_before = get_block(block_number="latest", parse=True)
    assert latest_block_before["status"] == "ACCEPTED_ON_L2"

    # some tx to generate a pending block, could be anything
    deploy_info = declare_and_deploy_with_chargeable(CONTRACT_PATH, inputs=["0"])

    # assert correct pending block
    pending_block = get_block(block_number="pending", parse=True)
    _assert_block_is_pending(pending_block)
    pending_tx_hashes = [tx["transaction_hash"] for tx in pending_block["transactions"]]
    assert deploy_info["tx_hash"] in pending_tx_hashes

    # assert latest unchanged
    latest_block = get_block(block_number="latest", parse=True)
    assert_equal(latest_block_before, latest_block)

    demand_block_creation()
    latest_block_after = get_block(block_number="latest", parse=True)
    assert pending_block["transactions"] == latest_block_after["transactions"]


@devnet_in_background("--blocks-on-demand")
def test_pending_block_traces():
    """Test that pending block traces contain pending data"""

    latest_block_traces_before = get_block_traces({"blockNumber": "latest"})

    # some tx to generate a pending block, could be anything
    deploy_info = declare_and_deploy_with_chargeable(CONTRACT_PATH, inputs=["0"])

    pending_block_traces = get_block_traces({"blockNumber": "pending"})
    assert len(pending_block_traces.traces) == 2
    assert_hex_equal(
        # trace at index 0 is declare, at index 1 is deploy
        hex(pending_block_traces.traces[1].transaction_hash),
        deploy_info["tx_hash"],
    )

    # assert latest unchanged
    latest_block_traces = get_block_traces({"blockNumber": "latest"})
    assert_equal(latest_block_traces_before, latest_block_traces)


@devnet_in_background("--blocks-on-demand")
def test_pending_state_update():
    """Test that pending state update contains pending data"""

    latest_state_update_before = get_state_update(block_number="latest")

    # some tx to generate a pending block, could be anything
    deploy_info = declare_and_deploy_with_chargeable(CONTRACT_PATH, inputs=["0"])

    pending_state_update = get_state_update(block_number="pending")
    pending_deployed = pending_state_update["state_diff"]["deployed_contracts"]
    assert_hex_equal(pending_deployed[0]["address"], deploy_info["address"])

    # assert latest unchanged
    latest_state_update = get_state_update(block_number="latest")
    assert_equal(latest_state_update_before, latest_state_update)


@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS, "--blocks-on-demand")
def test_events():
    """Test that events are stored and returned correctly in blocks-on-demand mode"""

    def get_events(to_block):
        resp = rpc_call(
            "starknet_getEvents",
            params={
                "filter": {
                    "address": rpc_felt(deploy_info["address"]),
                    "from_block": {"block_number": 0},
                    "to_block": to_block,
                    "chunk_size": 10,
                    "keys": [[rpc_felt(INCREASE_BALANCE_CALLED_EVENT_KEY)]],
                }
            },
        )

        if "result" in resp:
            # remove number and hash which hold improvised values for pending
            for event in resp["result"]["events"]:
                event.pop("block_number")
                event.pop("block_hash")

        return resp

    deploy_info = declare_and_deploy_with_chargeable(contract=EVENTS_CONTRACT_PATH)

    increase_arg = 123
    invoke(
        calls=[(deploy_info["address"], "increase_balance", [increase_arg])],
        account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
    )

    # genesis block is the latest at this point, and it is not expected to contain events
    latest_resp = get_events(to_block="latest")
    assert latest_resp["result"]["events"] == []
    assert get_events(to_block={"block_number": 1})["error"] == {
        "code": 24,
        "message": "Block not found",
    }

    # the pending block should contain the event emitted by events contract
    pending_resp = get_events(to_block="pending")
    pending_events = pending_resp["result"]["events"]
    assert len(pending_events) == 1
    assert increase_arg in map(lambda x: int(x, 16), pending_events[0]["data"])

    demand_block_creation()

    # newly created block should contain the same events as the pending block before it
    assert (
        get_events(to_block={"block_number": 1})["result"]["events"] == pending_events
    )
    assert get_events(to_block="latest")["result"]["events"] == pending_events

    # only one block should have been created
    assert get_events(to_block={"block_number": 2})["error"] == {
        "code": 24,
        "message": "Block not found",
    }


def _assert_correct_block_creation_response(resp: requests.Response):
    assert resp.status_code == 200
    resp_block_hash = resp.json()["block_hash"]
    latest_block = get_block(block_number="latest", parse=True)
    assert_hex_equal(resp_block_hash, latest_block["block_hash"])


@devnet_in_background("--blocks-on-demand")
def test_endpoint_if_no_pending():
    """Test block creation if no pending txs with on-demand flag set on"""
    resp = demand_block_creation()
    _assert_correct_block_creation_response(resp)


@devnet_in_background("--blocks-on-demand")
def test_endpoint_if_no_pending_after_creation():
    """
    Test block creation if no pending txs with on-demand flag set on
    and after one block has already been created
    """
    declare_and_deploy_with_chargeable(CONTRACT_PATH, inputs=["10"])
    resp = demand_block_creation()
    _assert_correct_block_creation_response(resp)

    resp2 = demand_block_creation()
    _assert_correct_block_creation_response(resp2)


@devnet_in_background("--blocks-on-demand")
def test_endpoint_for_successive_requests():
    """Send block creation request multiple times"""
    for _ in range(3):
        resp = demand_block_creation()
        _assert_correct_block_creation_response(resp)


@devnet_in_background()
def test_endpoint_without_on_demand_flag():
    """Test block creation with on-demand flag set off"""
    resp = demand_block_creation()
    _assert_correct_block_creation_response(resp)


@devnet_in_background("--blocks-on-demand")
def test_endpoint_if_some_pending():
    """Test block creation with some pending txs"""
    declare_and_deploy_with_chargeable(CONTRACT_PATH, inputs=["10"])
    resp = demand_block_creation()
    _assert_correct_block_creation_response(resp)


@devnet_in_background("--blocks-on-demand")
def test_increase_time_in_block_on_demand_mode():
    """Test block creation with increase_time and pending txs"""
    deploy_info = declare_and_deploy_with_chargeable(CONTRACT_PATH, inputs=["0"])
    assert_tx_status(deploy_info["tx_hash"], "PENDING")
    latest_block_timestamp = get_block(block_number="latest", parse=True)["timestamp"]

    # increase time should fail when there are pending transactions
    increase_time_response = increase_time(10000)
    assert increase_time_response.status_code == 400

    # increase time should succeed with no pending transactions
    resp = demand_block_creation()
    assert resp.status_code == 200
    increase_time_response = increase_time(10000)
    assert increase_time_response.status_code == 200
    latest_block = get_block(block_number="latest", parse=True)
    assert latest_block["timestamp"] >= latest_block_timestamp + 10000
    assert latest_block["block_hash"] == increase_time_response.json()["block_hash"]
    assert_tx_status(deploy_info["tx_hash"], "ACCEPTED_ON_L2")


@devnet_in_background("--blocks-on-demand")
def test_set_time_in_block_on_demand_mode():
    """Test block creation with set_time and pending txs"""
    deploy_info = declare_and_deploy_with_chargeable(CONTRACT_PATH, inputs=["0"])
    assert_tx_status(deploy_info["tx_hash"], "PENDING")
    latest_block_timestamp = get_block(block_number="latest", parse=True)["timestamp"]

    # set time should fail when there are pending transactions
    set_time_response = set_time(latest_block_timestamp + 10000)
    assert set_time_response.status_code == 400

    # set time should succeed with no pending transactions
    resp = demand_block_creation()
    assert resp.status_code == 200
    set_time_response = set_time(latest_block_timestamp + 10000)
    assert set_time_response.status_code == 200
    latest_block = get_block(block_number="latest", parse=True)
    assert latest_block["timestamp"] == latest_block_timestamp + 10000
    assert latest_block["block_hash"] == set_time_response.json()["block_hash"]
    assert_tx_status(deploy_info["tx_hash"], "ACCEPTED_ON_L2")
