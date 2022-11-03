"""Constants used across the project."""

try:
    from importlib_metadata import version
except ImportError:
    # >= py 3.8
    from importlib.metadata import version

CAIRO_LANG_VERSION = version("cairo-lang")
TIMEOUT_FOR_WEB3_REQUESTS = 120  # seconds
L1_MESSAGE_CANCELLATION_DELAY = (
    0  # Min amount of time in seconds for a message to be able to be cancelled
)

DEFAULT_HOST = "127.0.0.1"
DEFAULT_PORT = 5050

DEFAULT_ACCOUNTS = 10
DEFAULT_INITIAL_BALANCE = 10**21
DEFAULT_GAS_PRICE = 10**11

SUPPORTED_TX_VERSION = 1
SUPPORTED_RPC_TX_VERSION = 1

DUMMY_STATE_ROOT = bytes(32)

DEFAULT_TIMEOUT = 60  # seconds

OLD_SUPPORTED_VERSIONS = [0]

# account used by StarkNet CLI
OZ_ACCOUNT_CLASS_HASH = (
    895370652103566112291566439803611591116951595367594863638369163604569619773
)

LEGACY_RPC_TX_VERSION = 0
