"""Contains info about deployment of a test contract on alpha-goerli-2"""

from .shared import ALPHA_GOERLI2_URL, PREDEPLOY_ACCOUNT_CLI_ARGS

TESTNET_URL = ALPHA_GOERLI2_URL
TESTNET_CONTRACT_ADDRESS = (
    "0x32320dbdff79639db4ac0ff1f9f8b7450d31fee8ca1bccea7cfa0d7765fe0b2"
)
TESTNET_CONTRACT_SALT = (
    "0x10477367a9748e55196ab3c9ce04be74253cdb974e35a1d52ccda74d6d0e76b"
)
TESTNET_DEPLOYMENT_BLOCK = 8827  # this is when the contract was deployed
TESTNET_FORK_PARAMS = [*PREDEPLOY_ACCOUNT_CLI_ARGS, "--fork-network", "alpha-goerli2"]
