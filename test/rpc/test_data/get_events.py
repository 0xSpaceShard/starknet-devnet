"""
RPC get events test data.
"""

from test.shared import (
    EXPECTED_FEE_TOKEN_ADDRESS,
    PREDEPLOY_ACCOUNT_CLI_ARGS,
    PREDEPLOYED_ACCOUNT_ADDRESS,
)

BLOCK_0_LATEST = {
    "from_block": "0",
    "to_block": "latest",
    "chunk_size": 10,
}

BLOCK_0_2 = {
    "from_block": "0",
    "to_block": "2",
    "chunk_size": 10,
}

BLOCK_3_3 = {
    "from_block": "3",
    "to_block": "3",
    "chunk_size": 10,
}

BLOCK_2_3 = {
    "from_block": "2",
    "to_block": "3",
    "chunk_size": 10,
}

BLOCK_0_LATEST_CHUNK_3_0 = {
    "from_block": "0",
    "to_block": "latest",
    "chunk_size": 3,
}

BLOCK_0_LATEST_CHUNK_3_1 = {
    "from_block": "0",
    "to_block": "latest",
    "chunk_size": 3,
    "continuation_token": "1",
}

BLOCK_0_LATEST_ADDRESS = {
    "from_block": "0",
    "to_block": "latest",
    "address": EXPECTED_FEE_TOKEN_ADDRESS,
    "chunk_size": 10,
}

BLOCK_0_LATEST_KEY = {
    "from_block": "0",
    "to_block": "latest",
    "keys": ["0x99cd8bde557814842a3121e8ddfd433a539b8c9f14bf31ebf108d12e6196e9"],
    "chunk_size": 10,
}

BLOCK_0_LATEST_KEYS = {
    "from_block": "0",
    "to_block": "latest",
    "keys": [
        "0x99cd8bde557814842a3121e8ddfd433a539b8c9f14bf31ebf108d12e6196e9",
        "0x3db3da4221c078e78bd987e54e1cc24570d89a7002cefa33e548d6c72c73f9d",
    ],
    "chunk_size": 10,
}

BLOCK_0_LATEST_ADDRESS_KEYS = {
    "from_block": "0",
    "to_block": "latest",
    "address": EXPECTED_FEE_TOKEN_ADDRESS,
    "keys": [
        "0x99cd8bde557814842a3121e8ddfd433a539b8c9f14bf31ebf108d12e6196e9",
        "0x3db3da4221c078e78bd987e54e1cc24570d89a7002cefa33e548d6c72c73f9d",
    ],
    "chunk_size": 10,
}

EVENT_DATA_0 = [0, 0]
EVENT_DATA_1 = [0, 1]
EVENT_FEE_ADDRESS_2 = (
    1598625851760128517552627854997699631064626954749952456622017584404508471300
)
EVENT_FEE_1 = [
    int(PREDEPLOYED_ACCOUNT_ADDRESS, 16),
    EVENT_FEE_ADDRESS_2,
    143100000000000,  # WEI
    0,
]
EVENT_FEE_2 = [
    int(PREDEPLOYED_ACCOUNT_ADDRESS, 16),
    EVENT_FEE_ADDRESS_2,
    387900000000000,  # WEI
    0,
]

GET_EVENTS_TEST_DATA = [
    (
        [*PREDEPLOY_ACCOUNT_CLI_ARGS],
        BLOCK_0_LATEST,
        [EVENT_DATA_0, EVENT_FEE_1, EVENT_DATA_1, EVENT_FEE_2],
    ),
    (
        [*PREDEPLOY_ACCOUNT_CLI_ARGS],
        BLOCK_0_2,
        [EVENT_DATA_0, EVENT_FEE_1],
    ),
    (
        [*PREDEPLOY_ACCOUNT_CLI_ARGS],
        BLOCK_3_3,
        [EVENT_DATA_1, EVENT_FEE_2],
    ),
    (
        [*PREDEPLOY_ACCOUNT_CLI_ARGS],
        BLOCK_2_3,
        [EVENT_DATA_0, EVENT_FEE_1, EVENT_DATA_1, EVENT_FEE_2],
    ),
    (
        [*PREDEPLOY_ACCOUNT_CLI_ARGS],
        BLOCK_0_LATEST_CHUNK_3_0,
        [EVENT_DATA_0, EVENT_FEE_1, EVENT_DATA_1],
    ),
    (
        [*PREDEPLOY_ACCOUNT_CLI_ARGS],
        BLOCK_0_LATEST_CHUNK_3_1,
        [EVENT_FEE_2],
    ),
    (
        [*PREDEPLOY_ACCOUNT_CLI_ARGS],
        BLOCK_0_LATEST_ADDRESS,
        [EVENT_FEE_1, EVENT_FEE_2],
    ),
    (
        [*PREDEPLOY_ACCOUNT_CLI_ARGS],
        BLOCK_0_LATEST_KEY,
        [EVENT_FEE_1, EVENT_FEE_2],
    ),
    (
        [*PREDEPLOY_ACCOUNT_CLI_ARGS],
        BLOCK_0_LATEST_KEYS,
        [EVENT_DATA_0, EVENT_FEE_1, EVENT_DATA_1, EVENT_FEE_2],
    ),
    (
        [*PREDEPLOY_ACCOUNT_CLI_ARGS],
        BLOCK_0_LATEST_ADDRESS_KEYS,
        [EVENT_FEE_1, EVENT_FEE_2],
    ),
]
