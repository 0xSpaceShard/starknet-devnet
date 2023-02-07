"""
Test RPC schema validation
"""
from test.rpc.rpc_utils import rpc_call
from test.util import devnet_in_background
from unittest.mock import MagicMock, patch

import pytest
from starkware.starknet.public.abi import get_selector_from_name

from starknet_devnet.blueprints.rpc.schema import _assert_valid_rpc_request
from starknet_devnet.blueprints.rpc.structures.types import PredefinedRpcErrorCode
from starknet_devnet.blueprints.rpc.utils import rpc_felt


def _assert_call_does_not_raise_predefined_error(error_code: int):
    assert error_code not in [member.value for member in PredefinedRpcErrorCode]
    assert error_code in (
        20,
        21,
        22,
        24,
        40,
    )  # These are possible `starknet_call` error codes as of 0.2.1 spec


@pytest.mark.usefixtures("run_devnet_in_background")
@pytest.mark.parametrize(
    "params",
    (
        {
            "request": {
                "contract_address": "0x01",
                "entry_point_selector": rpc_felt(get_selector_from_name("get_balance")),
                "calldata": [],
            },
        },
        {
            "request": {
                "contract_address": "0x01",
                "entry_point_selector": rpc_felt(get_selector_from_name("get_balance")),
                "calldata": [],
            },
            "block_id": 12345,
        },
        {
            "request": {
                "contract_address": "0x01",
                "entry_point_selector": rpc_felt(get_selector_from_name("get_balance")),
                "calldata": [],
            },
            "block_id": "aaaeee",
        },
        {
            "block_id": "latest",
        },
        {},
        {
            "request": {
                "entry_point_selector": rpc_felt(get_selector_from_name("get_balance")),
                "calldata": [],
            },
            "block_id": "latest",
        },
        {
            "request": {
                "contract_address": "0x01",
                "entry_point_selector": rpc_felt(get_selector_from_name("get_balance")),
                "calldata": "0x1234",
            },
            "block_id": "latest",
        },
        {
            "request": {
                "contract_address": 1324,
                "entry_point_selector": rpc_felt(get_selector_from_name("get_balance")),
                "calldata": [],
            },
            "block_id": "latest",
        },
        {
            "request": {
                "contract_address": "0x01",
                "entry_point_selector": ["0x01", "0x02"],
                "calldata": [],
            },
            "block_id": "latest",
        },
    ),
)
def test_schema_raises_on_invalid_kwargs(params):
    """
    Call params validation with kwargs
    """
    resp = rpc_call("starknet_call", params=params)

    assert "error" in resp
    error = resp["error"]

    assert error["code"] == PredefinedRpcErrorCode.INVALID_PARAMS.value


@pytest.mark.usefixtures("run_devnet_in_background")
@pytest.mark.parametrize(
    "params",
    (
        [
            {
                "contract_address": "0x01",
                "entry_point_selector": rpc_felt(get_selector_from_name("get_balance")),
                "calldata": [],
            }
        ],
        [
            "latest",
        ],
        [
            {
                "contract_address": "0x01",
                "entry_point_selector": rpc_felt(get_selector_from_name("get_balance")),
                "calldata": [],
            },
            12345,
        ],
        [
            {
                "contract_address": "0x01",
                "entry_point_selector": rpc_felt(get_selector_from_name("get_balance")),
                "calldata": [],
            },
            "aaaeee",
        ],
        [],
        [
            {
                "entry_point_selector": rpc_felt(get_selector_from_name("get_balance")),
                "calldata": [],
            },
            "latest",
        ],
        [
            {
                "contract_address": "0x01",
                "entry_point_selector": rpc_felt(get_selector_from_name("get_balance")),
                "calldata": "0x1234",
            },
            "latest",
        ],
        [
            {
                "contract_address": 1324,
                "entry_point_selector": rpc_felt(get_selector_from_name("get_balance")),
                "calldata": [],
            },
            "latest",
        ],
        [
            {
                "contract_address": "0x01",
                "entry_point_selector": ["0x01", "0x02"],
                "calldata": [],
            },
            "latest",
        ],
        [
            "latest",
            {
                "contract_address": "0x01",
                "entry_point_selector": "0x01",
                "calldata": [],
            },
        ],
    ),
)
def test_schema_raises_on_invalid_args(params):
    """
    Test params validation with args
    """
    resp = rpc_call("starknet_call", params=params)

    assert "error" in resp
    error = resp["error"]

    assert error["code"] == PredefinedRpcErrorCode.INVALID_PARAMS.value


@devnet_in_background("--disable-rpc-request-validation")
def test_schema_does_not_raise_on_disabled_request_validation():
    """
    Test schema validation is disabled by CLI argument
    """
    resp = rpc_call(
        "starknet_call",
        params={
            "block_id": "latest",
            "request": {
                "contract_address": 1234,
                "entry_point_selector": -1,
                "calldata": ["a", "b", "c"],
            },
        },
    )

    # Error will be raised when trying to execute function, but it shouldn't be the INVALID_PARAMS error
    error = resp["error"]
    code = error["code"]
    _assert_call_does_not_raise_predefined_error(code)


@pytest.mark.usefixtures("run_devnet_in_background")
@pytest.mark.parametrize(
    "params",
    (
        {
            "request": {
                "contract_address": "0x01",
                "entry_point_selector": rpc_felt(get_selector_from_name("get_balance")),
                "calldata": [],
            },
            "block_id": "latest",
        },
        {
            "block_id": "latest",
            "request": {
                "contract_address": "0x01",
                "entry_point_selector": rpc_felt(get_selector_from_name("get_balance")),
                "calldata": [],
            },
        },
        {
            "request": {
                "contract_address": "0x01",
                "entry_point_selector": rpc_felt(get_selector_from_name("get_balance")),
                "calldata": [],
            },
            "block_id": {"block_hash": "0x00"},
        },
        {
            "request": {
                "contract_address": "0x01",
                "entry_point_selector": rpc_felt(get_selector_from_name("get_balance")),
                "calldata": [],
            },
            "block_id": {
                "block_number": 0,
            },
        },
    ),
)
def test_schema_does_not_raise_on_correct_kwargs(params):
    """
    Test kwargs validation allows valid requests
    """

    resp = rpc_call("starknet_call", params=params)

    # Error will be raised because address is correctly formatted but incorrect
    error = resp["error"]
    code = error["code"]
    _assert_call_does_not_raise_predefined_error(code)


@pytest.mark.usefixtures("run_devnet_in_background")
def test_schema_does_not_raise_on_correct_args():
    """
    Test args validation allows valid requests
    """

    resp = rpc_call(
        "starknet_call",
        params=[
            {
                "contract_address": "0x01",
                "entry_point_selector": rpc_felt(get_selector_from_name("get_balance")),
                "calldata": [],
            },
            "latest",
        ],
    )

    # Error will be raised because address is correctly formatted but incorrect
    error = resp["error"]
    code = error["code"]
    _assert_call_does_not_raise_predefined_error(code)


def test_schema_with_optional_values():
    """
    Test schema validation allowing omitting non-required values.

    StarkNet JSON RPC spec methods params have a "required": bool field alongside the schema.
    Schema validation must support handling these optional parameters.

    This test mocks an internal method, as I found no other solution working with e2e tests format we have currently.
    In this internal representation, "required" is translated to "is_required": bool alongside
    with the rest of the schema.
    """
    with patch(
        "starknet_devnet.blueprints.rpc.schema._request_schemas_for_method", MagicMock()
    ) as mocked:
        mocked.return_value = {
            "key": {
                "is_required": True,
                "$ref": "#/components/schemas/STORAGE_KEY",
                "components": {
                    "schemas": {
                        "STORAGE_KEY": {
                            "type": "string",
                            "title": "A storage key",
                            "$comment": "A storage key, represented as a string of hex digits",
                            "description": "A storage key. Represented as up to 62 hex digits, 3 bits, and 5 leading zeroes.",
                            "pattern": "^0x0[0-7]{1}[a-fA-F0-9]{0,62}$",
                        }
                    }
                },
            },
            "value": {
                "is_required": False,
                "$ref": "#/components/schemas/STORAGE_KEY",
                "components": {
                    "schemas": {
                        "STORAGE_KEY": {
                            "type": "string",
                            "title": "A storage key",
                            "$comment": "A storage key, represented as a string of hex digits",
                            "description": "A storage key. Represented as up to 62 hex digits, 3 bits, and 5 leading zeroes.",
                            "pattern": "^0x0[0-7]{1}[a-fA-F0-9]{0,62}$",
                        }
                    }
                },
            },
        }

        params = {"key": "0x01"}
        _assert_valid_rpc_request(**params, method_name="starknet_method")
