"""
RPC get events test data.
"""

from test.shared import (
    EXPECTED_FEE_TOKEN_ADDRESS,
    PREDEPLOY_ACCOUNT_CLI_ARGS,
    PREDEPLOYED_ACCOUNT_ADDRESS,
)

BLOCK_FROM_0_TO_LATEST = {
    "from_block": "0",
    "to_block": "latest",
    "chunk_size": 10,
}

BLOCK_FROM_0_TO_2 = {
    "from_block": "0",
    "to_block": "2",
    "chunk_size": 10,
}

BLOCK_FROM_3_TO_3 = {
    "from_block": "3",
    "to_block": "3",
    "chunk_size": 10,
}

BLOCK_FROM_2_TO_3 = {
    "from_block": "2",
    "to_block": "3",
    "chunk_size": 10,
}

BLOCK_FROM_0_TO_LATEST_CHUNK_SIZE_3 = {
    "from_block": "0",
    "to_block": "latest",
    "chunk_size": 3,
}

BLOCK_FROM_0_TO_LATEST_CHUNK_3_CONTINUATION_TOKEN = {
    "from_block": "0",
    "to_block": "latest",
    "chunk_size": 3,
    "continuation_token": "1",
}

BLOCK_FROM_0_TO_LATEST_WITH_ADDRESS = {
    "from_block": "0",
    "to_block": "latest",
    "address": EXPECTED_FEE_TOKEN_ADDRESS,
    "chunk_size": 10,
}

BLOCK_FROM_0_TO_LATEST_WITH_KEY = {
    "from_block": "0",
    "to_block": "latest",
    "keys": ["0x99cd8bde557814842a3121e8ddfd433a539b8c9f14bf31ebf108d12e6196e9"],
    "chunk_size": 10,
}

BLOCK_FROM_0_TO_LATEST_WITH_KEYS = {
    "from_block": "0",
    "to_block": "latest",
    "keys": [
        "0x99cd8bde557814842a3121e8ddfd433a539b8c9f14bf31ebf108d12e6196e9",
        "0x3db3da4221c078e78bd987e54e1cc24570d89a7002cefa33e548d6c72c73f9d",
    ],
    "chunk_size": 10,
}

BLOCK_FROM_0_TO_LATEST_WITH_ADDRESS_AND_KEYS = {
    "from_block": "0",
    "to_block": "latest",
    "address": EXPECTED_FEE_TOKEN_ADDRESS,
    "keys": [
        "0x99cd8bde557814842a3121e8ddfd433a539b8c9f14bf31ebf108d12e6196e9",
        "0x3db3da4221c078e78bd987e54e1cc24570d89a7002cefa33e548d6c72c73f9d",
    ],
    "chunk_size": 10,
}

EVENT_INCREASE_BALANCE_BY_0 = [0, 0]
EVENT_INCREASE_BALANCE_BY_1 = [0, 1]
EVENT_FEE_ADDRESS = (
    1598625851760128517552627854997699631064626954749952456622017584404508471300
)
EVENT_FEE_ESTIMATION_IN_BLOCK_1 = [
    int(PREDEPLOYED_ACCOUNT_ADDRESS, 16),
    EVENT_FEE_ADDRESS,
    143200000000000,  # WEI
    0,
]
EVENT_FEE_ESTIMATION_IN_BLOCK_2 = [
    int(PREDEPLOYED_ACCOUNT_ADDRESS, 16),
    EVENT_FEE_ADDRESS,
    388000000000000,  # WEI
    0,
]

GET_EVENTS_TEST_DATA = [
    (
        [*PREDEPLOY_ACCOUNT_CLI_ARGS],
        BLOCK_FROM_0_TO_LATEST,
        [
            EVENT_INCREASE_BALANCE_BY_0,
            EVENT_FEE_ESTIMATION_IN_BLOCK_1,
            EVENT_INCREASE_BALANCE_BY_1,
            EVENT_FEE_ESTIMATION_IN_BLOCK_2,
        ],
    ),
    (
        [*PREDEPLOY_ACCOUNT_CLI_ARGS],
        BLOCK_FROM_0_TO_2,
        [EVENT_INCREASE_BALANCE_BY_0, EVENT_FEE_ESTIMATION_IN_BLOCK_1],
    ),
    (
        [*PREDEPLOY_ACCOUNT_CLI_ARGS],
        BLOCK_FROM_3_TO_3,
        [EVENT_INCREASE_BALANCE_BY_1, EVENT_FEE_ESTIMATION_IN_BLOCK_2],
    ),
    (
        [*PREDEPLOY_ACCOUNT_CLI_ARGS],
        BLOCK_FROM_2_TO_3,
        [
            EVENT_INCREASE_BALANCE_BY_0,
            EVENT_FEE_ESTIMATION_IN_BLOCK_1,
            EVENT_INCREASE_BALANCE_BY_1,
            EVENT_FEE_ESTIMATION_IN_BLOCK_2,
        ],
    ),
    (
        [*PREDEPLOY_ACCOUNT_CLI_ARGS],
        BLOCK_FROM_0_TO_LATEST_CHUNK_SIZE_3,
        [
            EVENT_INCREASE_BALANCE_BY_0,
            EVENT_FEE_ESTIMATION_IN_BLOCK_1,
            EVENT_INCREASE_BALANCE_BY_1,
        ],
    ),
    (
        [*PREDEPLOY_ACCOUNT_CLI_ARGS],
        BLOCK_FROM_0_TO_LATEST_CHUNK_3_CONTINUATION_TOKEN,
        [EVENT_FEE_ESTIMATION_IN_BLOCK_2],
    ),
    (
        [*PREDEPLOY_ACCOUNT_CLI_ARGS],
        BLOCK_FROM_0_TO_LATEST_WITH_ADDRESS,
        [EVENT_FEE_ESTIMATION_IN_BLOCK_1, EVENT_FEE_ESTIMATION_IN_BLOCK_2],
    ),
    (
        [*PREDEPLOY_ACCOUNT_CLI_ARGS],
        BLOCK_FROM_0_TO_LATEST_WITH_KEY,
        [EVENT_FEE_ESTIMATION_IN_BLOCK_1, EVENT_FEE_ESTIMATION_IN_BLOCK_2],
    ),
    (
        [*PREDEPLOY_ACCOUNT_CLI_ARGS],
        BLOCK_FROM_0_TO_LATEST_WITH_KEYS,
        [
            EVENT_INCREASE_BALANCE_BY_0,
            EVENT_FEE_ESTIMATION_IN_BLOCK_1,
            EVENT_INCREASE_BALANCE_BY_1,
            EVENT_FEE_ESTIMATION_IN_BLOCK_2,
        ],
    ),
    (
        [*PREDEPLOY_ACCOUNT_CLI_ARGS],
        BLOCK_FROM_0_TO_LATEST_WITH_ADDRESS_AND_KEYS,
        [EVENT_FEE_ESTIMATION_IN_BLOCK_1, EVENT_FEE_ESTIMATION_IN_BLOCK_2],
    ),
]
