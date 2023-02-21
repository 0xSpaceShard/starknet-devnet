"""
Test genesis block
"""

import pytest

from .util import get_block

FEE_TOKEN_CLASS_HASH = (
    "0x6a22bf63c7bc07effa39a25dfbd21523d211db0100a0afd054d172b81840eaf"
)
UDC_CLASS_HASH = "0x7b3e05f48f0c69e4a65ce5e076a66271a527aff2c34ce1083ec6e1526997a69"
CHARGEABLE_ACCOUNT_CLASS_HASH = (
    "0x4d07e40e93398ed3c76981e72dd1fd22557a78ce36c0515f679e27f0bb5bc5f"
)
STARKNET_CLI_ACCOUNT_CLASS_HASH = (
    "0x6f500f527355dfdb8093c7fe46e6f73c96a867392b49fa4157a757538928539"
)


@pytest.fixture(name="last_tx_hash")
def fixture_last_tx_hash(request):
    """
    Fixture for last tx_hash
    """
    return request.param


@pytest.mark.usefixtures("run_devnet_in_background")
@pytest.mark.parametrize(
    "run_devnet_in_background, last_tx_hash",
    [
        (
            ["--accounts", "1"],
            8,
        ),
        (
            ["--accounts", "10"],
            17,
        ),
    ],
    indirect=True,
)
def test_genesis_block_transactions(last_tx_hash):
    """
    Test genesis block transactions after devnet start.
    There are 4 declare and 3 deploy transactions + accounts deploy depending on the number of accounts.
    """

    genesis_block = get_block(block_number="latest", parse=True)

    # Assert transaction iterator with genesis block transactions
    assert len(genesis_block["transactions"]) == last_tx_hash

    # Assert class hashes in the first 4 declare transactions
    declared = set(tx["class_hash"] for tx in genesis_block["transactions"][:4])
    expected_declared = set(
        [
            FEE_TOKEN_CLASS_HASH,
            UDC_CLASS_HASH,
            CHARGEABLE_ACCOUNT_CLASS_HASH,
            STARKNET_CLI_ACCOUNT_CLASS_HASH,
        ]
    )
    assert declared == expected_declared

    # Assert transaction hashes for all transactions
    for i in range(0, last_tx_hash):
        assert genesis_block["transactions"][i]["transaction_hash"] == hex(i + 1)
