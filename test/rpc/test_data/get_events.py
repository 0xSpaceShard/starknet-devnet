"""
RPC get events test data.
"""
from test.shared import (
    EXPECTED_FEE_TOKEN_ADDRESS,
    FEE_CHARGED_EVENT_KEY,
    INCREASE_BALANCE_CALLED_EVENT_KEY,
    PREDEPLOY_ACCOUNT_CLI_ARGS,
    PREDEPLOYED_ACCOUNT_ADDRESS,
)

from starknet_devnet.blueprints.rpc.utils import rpc_felt

BLOCK_FROM_0_TO_LATEST = {
    "from_block": {"block_number": 0},
    "address": rpc_felt(EXPECTED_FEE_TOKEN_ADDRESS),
    "keys": [
        rpc_felt(FEE_CHARGED_EVENT_KEY),
        rpc_felt(INCREASE_BALANCE_CALLED_EVENT_KEY),
    ],
    "to_block": "latest",
    "chunk_size": 10,
}

BLOCK_FROM_0x0_TO_LATEST = {
    "from_block": {"block_hash": rpc_felt("0x0")},
    "address": rpc_felt(EXPECTED_FEE_TOKEN_ADDRESS),
    "keys": [
        rpc_felt(FEE_CHARGED_EVENT_KEY),
    ],
    "to_block": "latest",
    "chunk_size": 10,
}

BLOCK_FROM_0_TO_LATEST_MALFORMED_REQUEST = {
    "from_block": {"block_number": 0},
    "to_block": "latest",
    "address": rpc_felt(EXPECTED_FEE_TOKEN_ADDRESS),
    "keys": [
        rpc_felt(FEE_CHARGED_EVENT_KEY),
    ],
    "chunk_size": "test",
}

BLOCK_FROM_0_TO_LATEST_MISSING_PARAMETER = {
    "from_block": {"block_number": 0},
    "keys": [
        rpc_felt(FEE_CHARGED_EVENT_KEY),
    ],
    "to_block": "latest",
    "chunk_size": 10,
}

BLOCK_FROM_0_TO_LATEST_WRONG_BLOCK_TYPE = {
    "from_block": {"block_number": "0x0"},
    "address": rpc_felt(EXPECTED_FEE_TOKEN_ADDRESS),
    "keys": [
        rpc_felt(FEE_CHARGED_EVENT_KEY),
    ],
    "to_block": "latest",
    "chunk_size": 10,
}

BLOCK_FROM_0_TO_2 = {
    "from_block": {"block_hash": rpc_felt("0x0")},
    "to_block": {"block_number": 2},
    "address": rpc_felt(EXPECTED_FEE_TOKEN_ADDRESS),
    "keys": [
        rpc_felt(FEE_CHARGED_EVENT_KEY),
    ],
    "chunk_size": 10,
}

BLOCK_FROM_3_TO_3 = {
    "from_block": {"block_number": 3},
    "to_block": {"block_number": 3},
    "address": rpc_felt(EXPECTED_FEE_TOKEN_ADDRESS),
    "keys": [
        rpc_felt(FEE_CHARGED_EVENT_KEY),
    ],
    "chunk_size": 10,
}

BLOCK_FROM_2_TO_3 = {
    "from_block": {"block_number": 2},
    "to_block": {"block_number": 3},
    "address": rpc_felt(EXPECTED_FEE_TOKEN_ADDRESS),
    "keys": [
        rpc_felt(FEE_CHARGED_EVENT_KEY),
    ],
    "chunk_size": 10,
}

BLOCK_FROM_0_TO_LATEST_CHUNK_SIZE_1 = {
    "from_block": {"block_number": 0},
    "address": rpc_felt(EXPECTED_FEE_TOKEN_ADDRESS),
    "keys": [
        rpc_felt(FEE_CHARGED_EVENT_KEY),
    ],
    "to_block": "latest",
    "chunk_size": 1,
}

BLOCK_FROM_0_TO_LATEST_CHUNK_1_CONTINUATION_TOKEN = {
    "from_block": {"block_number": 0},
    "to_block": "latest",
    "address": rpc_felt(EXPECTED_FEE_TOKEN_ADDRESS),
    "keys": [
        rpc_felt(FEE_CHARGED_EVENT_KEY),
    ],
    "chunk_size": 1,
    "continuation_token": "1",
}

BLOCK_FROM_0_TO_1_CHUNK_3_CONTINUATION_TOKEN = {
    "from_block": {"block_number": 0},
    "to_block": {"block_number": 1},
    "address": rpc_felt(EXPECTED_FEE_TOKEN_ADDRESS),
    "keys": [
        rpc_felt(FEE_CHARGED_EVENT_KEY),
    ],
    "chunk_size": 3,
    "continuation_token": "0",
}

EVENT_FEE_ADDRESS = rpc_felt(
    0xBEBE7DEC64B911AEFFECC184AFCEFA6470E3B3652A1605E42D643E1EA9093D
)
FEE_CHARGING_IN_BLOCK_2_EVENT = [
    rpc_felt(int(PREDEPLOYED_ACCOUNT_ADDRESS, 16)),
    EVENT_FEE_ADDRESS,
    rpc_felt(0x73B00ED0C000),  # WEI
    rpc_felt(0),
]
FEE_CHARGING_IN_BLOCK_3_EVENT = [
    rpc_felt(int(PREDEPLOYED_ACCOUNT_ADDRESS, 16)),
    EVENT_FEE_ADDRESS,
    rpc_felt(0x015254FFDB4000),  # WEI
    rpc_felt(0),
]


def create_get_events_rpc(filter_data):
    """
    Construct JSON RPC call with filter data
    """
    return {
        "jsonrpc": "2.0",
        "method": "starknet_getEvents",
        "params": {"filter": filter_data},
        "id": 0,
    }


GET_EVENTS_TEST_DATA = [
    (
        [*PREDEPLOY_ACCOUNT_CLI_ARGS],
        create_get_events_rpc(BLOCK_FROM_0_TO_LATEST),
        [
            FEE_CHARGING_IN_BLOCK_2_EVENT,
            FEE_CHARGING_IN_BLOCK_3_EVENT,
        ],
    ),
    (
        [*PREDEPLOY_ACCOUNT_CLI_ARGS],
        create_get_events_rpc(BLOCK_FROM_0x0_TO_LATEST),
        [
            FEE_CHARGING_IN_BLOCK_2_EVENT,
            FEE_CHARGING_IN_BLOCK_3_EVENT,
        ],
    ),
    (
        [*PREDEPLOY_ACCOUNT_CLI_ARGS],
        create_get_events_rpc(BLOCK_FROM_0_TO_2),
        [FEE_CHARGING_IN_BLOCK_2_EVENT],
    ),
    (
        [*PREDEPLOY_ACCOUNT_CLI_ARGS],
        create_get_events_rpc(BLOCK_FROM_3_TO_3),
        [FEE_CHARGING_IN_BLOCK_3_EVENT],
    ),
    (
        [*PREDEPLOY_ACCOUNT_CLI_ARGS],
        create_get_events_rpc(BLOCK_FROM_2_TO_3),
        [
            FEE_CHARGING_IN_BLOCK_2_EVENT,
            FEE_CHARGING_IN_BLOCK_3_EVENT,
        ],
    ),
    (
        [*PREDEPLOY_ACCOUNT_CLI_ARGS],
        create_get_events_rpc(BLOCK_FROM_0_TO_LATEST_CHUNK_SIZE_1),
        [
            FEE_CHARGING_IN_BLOCK_2_EVENT,
        ],
    ),
    (
        [*PREDEPLOY_ACCOUNT_CLI_ARGS],
        create_get_events_rpc(BLOCK_FROM_0_TO_LATEST_CHUNK_1_CONTINUATION_TOKEN),
        [FEE_CHARGING_IN_BLOCK_3_EVENT],
    ),
    (
        [*PREDEPLOY_ACCOUNT_CLI_ARGS],
        create_get_events_rpc(BLOCK_FROM_0_TO_1_CHUNK_3_CONTINUATION_TOKEN),
        [],
    ),
]
