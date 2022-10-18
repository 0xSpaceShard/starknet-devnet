"""
RPC get events test data.
"""

from test.shared import PREDEPLOY_ACCOUNT_CLI_ARGS

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
    "address": "0x62230eA046a9a5fbc261ac77d03c8d41e5d442db2284587570ab46455fd2488",
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
    "address": "0x62230eA046a9a5fbc261ac77d03c8d41e5d442db2284587570ab46455fd2488",
    "keys": [
        "0x99cd8bde557814842a3121e8ddfd433a539b8c9f14bf31ebf108d12e6196e9",
        "0x3db3da4221c078e78bd987e54e1cc24570d89a7002cefa33e548d6c72c73f9d",
    ],
    "chunk_size": 10,
}

EVENT_DATA_0 = [0, 0]
EVENT_DATA_1 = [0, 1]
EVENT_FEE_ADDRESS_1 = (
    1483697464188569092488551792700745002099692778312526338015109648680562628122
)
EVENT_FEE_ADDRESS_2 = (
    3061382497757462279541465773886495848031317811525138643618478891350337006185
)
EVENT_FEE_WEI = 387800000000000
EVENT_FEE = [EVENT_FEE_ADDRESS_1, EVENT_FEE_ADDRESS_2, EVENT_FEE_WEI, 0]

GET_EVENTS_TEST_DATA = [
    (
        [*PREDEPLOY_ACCOUNT_CLI_ARGS],
        BLOCK_0_LATEST,
        [EVENT_DATA_0, EVENT_FEE, EVENT_DATA_1, EVENT_FEE],
    ),
    (
        [*PREDEPLOY_ACCOUNT_CLI_ARGS],
        BLOCK_0_2,
        [EVENT_DATA_0, EVENT_FEE],
    ),
    (
        [*PREDEPLOY_ACCOUNT_CLI_ARGS],
        BLOCK_3_3,
        [EVENT_DATA_1, EVENT_FEE],
    ),
    (
        [*PREDEPLOY_ACCOUNT_CLI_ARGS],
        BLOCK_2_3,
        [EVENT_DATA_0, EVENT_FEE, EVENT_DATA_1, EVENT_FEE],
    ),
    (
        [*PREDEPLOY_ACCOUNT_CLI_ARGS],
        BLOCK_0_LATEST_CHUNK_3_0,
        [EVENT_DATA_0, EVENT_FEE, EVENT_DATA_1],
    ),
    (
        [*PREDEPLOY_ACCOUNT_CLI_ARGS],
        BLOCK_0_LATEST_CHUNK_3_1,
        [EVENT_FEE],
    ),
    (
        [*PREDEPLOY_ACCOUNT_CLI_ARGS],
        BLOCK_0_LATEST_ADDRESS,
        [EVENT_FEE, EVENT_FEE],
    ),
    (
        [*PREDEPLOY_ACCOUNT_CLI_ARGS],
        BLOCK_0_LATEST_KEY,
        [EVENT_FEE, EVENT_FEE],
    ),
    (
        [*PREDEPLOY_ACCOUNT_CLI_ARGS],
        BLOCK_0_LATEST_KEYS,
        [EVENT_DATA_0, EVENT_FEE, EVENT_DATA_1, EVENT_FEE],
    ),
    (
        [*PREDEPLOY_ACCOUNT_CLI_ARGS],
        BLOCK_0_LATEST_ADDRESS_KEYS,
        [EVENT_FEE, EVENT_FEE],
    ),
]
