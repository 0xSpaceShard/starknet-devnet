"""
Contains general_config generation functionalities.
"""
from enum import Enum
from starkware.starknet.definitions import constants
from starkware.starknet.definitions.general_config import (
    DEFAULT_CHAIN_ID,
    DEFAULT_GAS_PRICE,
    DEFAULT_MAX_STEPS,
    DEFAULT_SEQUENCER_ADDRESS,
    DEFAULT_VALIDATE_MAX_STEPS,
    build_general_config,
)

from .constants import SUPPORTED_TX_VERSION
from .fee_token import FeeToken
from starkware.python.utils import from_bytes

class StarknetChainId(Enum):
    MAINNET = from_bytes(b"SN_MAIN")
    TESTNET = from_bytes(b"SN_GOERLI")

def build_general_config_chain_id(chain_id):

    # Just for tests - remove later
    print("StarknetChainId[chain_id].name if chain_id else DEFAULT_CHAIN_ID.name")
    print(StarknetChainId[chain_id].name if chain_id else DEFAULT_CHAIN_ID.name)
    
    return build_general_config(
        {
            "cairo_resource_fee_weights": {
                "n_steps": constants.N_STEPS_FEE_WEIGHT,
            },
            "contract_storage_commitment_tree_height": constants.CONTRACT_STATES_COMMITMENT_TREE_HEIGHT,
            "event_commitment_tree_height": constants.EVENT_COMMITMENT_TREE_HEIGHT,
            "global_state_commitment_tree_height": constants.CONTRACT_ADDRESS_BITS,
            "invoke_tx_max_n_steps": DEFAULT_MAX_STEPS,
            "min_gas_price": DEFAULT_GAS_PRICE,
            "sequencer_address": hex(DEFAULT_SEQUENCER_ADDRESS),
            "starknet_os_config": {
                "chain_id": StarknetChainId[chain_id].name if chain_id else DEFAULT_CHAIN_ID.name, 
                "fee_token_address": hex(FeeToken.ADDRESS),
            },
            "tx_version": SUPPORTED_TX_VERSION,
            "tx_commitment_tree_height": constants.TRANSACTION_COMMITMENT_TREE_HEIGHT,
            "validate_max_n_steps": DEFAULT_VALIDATE_MAX_STEPS,
        }
    )

# Remove or unify later
DEFAULT_GENERAL_CONFIG = build_general_config(
    {
        "cairo_resource_fee_weights": {
            "n_steps": constants.N_STEPS_FEE_WEIGHT,
        },
        "contract_storage_commitment_tree_height": constants.CONTRACT_STATES_COMMITMENT_TREE_HEIGHT,
        "event_commitment_tree_height": constants.EVENT_COMMITMENT_TREE_HEIGHT,
        "global_state_commitment_tree_height": constants.CONTRACT_ADDRESS_BITS,
        "invoke_tx_max_n_steps": DEFAULT_MAX_STEPS,
        "min_gas_price": DEFAULT_GAS_PRICE,
        "sequencer_address": hex(DEFAULT_SEQUENCER_ADDRESS),
        "starknet_os_config": {
            "chain_id": DEFAULT_CHAIN_ID.name,
            "fee_token_address": hex(FeeToken.ADDRESS),
        },
        "tx_version": SUPPORTED_TX_VERSION,
        "tx_commitment_tree_height": constants.TRANSACTION_COMMITMENT_TREE_HEIGHT,
        "validate_max_n_steps": DEFAULT_VALIDATE_MAX_STEPS,
    }
)
