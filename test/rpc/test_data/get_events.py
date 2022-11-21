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

BLOCK_FROM_0_TO_1_CHUNK_3_CONTINUATION_TOKEN = {
    "from_block": "0",
    "to_block": "1",
    "chunk_size": 3,
    "continuation_token": "0",
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

INCREASE_BALANCE_BY_0_EVENT = [0, 0]
INCREASE_BALANCE_BY_1_EVENT = [0, 1]
EVENT_FEE_ADDRESS = (
    1848132115085043480193496279185058626650378119257763769763822937432943340424
)
FEE_CHARGING_IN_BLOCK_2_EVENT = [
    int(PREDEPLOYED_ACCOUNT_ADDRESS, 16),
    EVENT_FEE_ADDRESS,
    143300000000000,  # WEI
    0,
]
FEE_CHARGING_IN_BLOCK_3_EVENT = [
    int(PREDEPLOYED_ACCOUNT_ADDRESS, 16),
    EVENT_FEE_ADDRESS,
    388100000000000,  # WEI
    0,
]

GET_EVENTS_TEST_DATA = [
    (
        [*PREDEPLOY_ACCOUNT_CLI_ARGS],
        BLOCK_FROM_0_TO_LATEST,
        [
            INCREASE_BALANCE_BY_0_EVENT,
            FEE_CHARGING_IN_BLOCK_2_EVENT,
            INCREASE_BALANCE_BY_1_EVENT,
            FEE_CHARGING_IN_BLOCK_3_EVENT,
        ],
    ),
    (
        [*PREDEPLOY_ACCOUNT_CLI_ARGS],
        BLOCK_FROM_0_TO_2,
        [INCREASE_BALANCE_BY_0_EVENT, FEE_CHARGING_IN_BLOCK_2_EVENT],
    ),
    (
        [*PREDEPLOY_ACCOUNT_CLI_ARGS],
        BLOCK_FROM_3_TO_3,
        [INCREASE_BALANCE_BY_1_EVENT, FEE_CHARGING_IN_BLOCK_3_EVENT],
    ),
    (
        [*PREDEPLOY_ACCOUNT_CLI_ARGS],
        BLOCK_FROM_2_TO_3,
        [
            INCREASE_BALANCE_BY_0_EVENT,
            FEE_CHARGING_IN_BLOCK_2_EVENT,
            INCREASE_BALANCE_BY_1_EVENT,
            FEE_CHARGING_IN_BLOCK_3_EVENT,
        ],
    ),
    (
        [*PREDEPLOY_ACCOUNT_CLI_ARGS],
        BLOCK_FROM_0_TO_LATEST_CHUNK_SIZE_3,
        [
            INCREASE_BALANCE_BY_0_EVENT,
            FEE_CHARGING_IN_BLOCK_2_EVENT,
            INCREASE_BALANCE_BY_1_EVENT,
        ],
    ),
    (
        [*PREDEPLOY_ACCOUNT_CLI_ARGS],
        BLOCK_FROM_0_TO_LATEST_CHUNK_3_CONTINUATION_TOKEN,
        [FEE_CHARGING_IN_BLOCK_3_EVENT],
    ),
    (
        [*PREDEPLOY_ACCOUNT_CLI_ARGS],
        BLOCK_FROM_0_TO_1_CHUNK_3_CONTINUATION_TOKEN,
        [],
    ),
    (
        [*PREDEPLOY_ACCOUNT_CLI_ARGS],
        BLOCK_FROM_0_TO_LATEST_WITH_ADDRESS,
        [FEE_CHARGING_IN_BLOCK_2_EVENT, FEE_CHARGING_IN_BLOCK_3_EVENT],
    ),
    (
        [*PREDEPLOY_ACCOUNT_CLI_ARGS],
        BLOCK_FROM_0_TO_LATEST_WITH_KEY,
        [FEE_CHARGING_IN_BLOCK_2_EVENT, FEE_CHARGING_IN_BLOCK_3_EVENT],
    ),
    (
        [*PREDEPLOY_ACCOUNT_CLI_ARGS],
        BLOCK_FROM_0_TO_LATEST_WITH_KEYS,
        [
            INCREASE_BALANCE_BY_0_EVENT,
            FEE_CHARGING_IN_BLOCK_2_EVENT,
            INCREASE_BALANCE_BY_1_EVENT,
            FEE_CHARGING_IN_BLOCK_3_EVENT,
        ],
    ),
    (
        [*PREDEPLOY_ACCOUNT_CLI_ARGS],
        BLOCK_FROM_0_TO_LATEST_WITH_ADDRESS_AND_KEYS,
        [FEE_CHARGING_IN_BLOCK_2_EVENT, FEE_CHARGING_IN_BLOCK_3_EVENT],
    ),
]
