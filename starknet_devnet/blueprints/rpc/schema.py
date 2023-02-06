"""
Utilities for validating RPC responses against RPC specification
"""
import json
from collections import OrderedDict
from functools import lru_cache, wraps
from itertools import zip_longest
from typing import Any, Dict, List
from typing import OrderedDict as OrderedDictType
from typing import Tuple

from jsonschema import validate
from jsonschema.exceptions import ValidationError

from starknet_devnet.blueprints.rpc.rpc_spec import RPC_SPECIFICATION
from starknet_devnet.blueprints.rpc.rpc_spec_write import RPC_SPECIFICATION_WRITE
from starknet_devnet.state import state


# Cache the function result so schemas are not reloaded from disk on every call
@lru_cache
def _load_schemas() -> Tuple[Dict[str, Any], Dict[str, Any]]:
    specs_json = json.loads(RPC_SPECIFICATION)
    write_specs_json = json.loads(RPC_SPECIFICATION_WRITE)
    schemas = specs_json["components"]["schemas"]

    methods = {**_extract_methods(specs_json), **_extract_methods(write_specs_json)}

    for schema in schemas.values():
        # Newer version of the RPC (above 0.45.0) has properly defined `required` fields.
        # Once we start targeting them, this can be removed.
        #
        # NOTE: This does not add `required` to all schemas that should have it, i.e. it fails to add `required`
        # to nested objects. This causes validation to be incomplete in these cases.
        if "required" not in schema and "properties" in schema:
            schema["required"] = list(schema["properties"].keys())

    return methods, schemas


def _extract_methods(specs_json: Dict[str, Any]) -> Dict[str, Any]:
    return {method["name"]: method for method in specs_json["methods"]}


@lru_cache
def _response_schema_for_method(name: str) -> Dict[str, Any]:
    """
    Return a dict with correct structure for jsonschema validation.

    {
        // base schema
        // ...
        "components": {
            "schemas": {
                // rest of the schemas
                // ...
            }
        }
    }

    Main "base schema" has to be placed in the "top-level" of the dictionary, so jsonschema
    knows against what of the multiple schemas present in the RPC spec to verify.

    RPC spec is currently formatted in the way, that every `$ref` is made with respect to
    #/components/schemas/SCHEMA_NAME
    The dict with schemas has to follow the same structure using
    nested dictionaries.
    """

    methods, schemas = _load_schemas()
    base_schema = methods[name]["result"]["schema"]
    base_schema["required"] = methods[name]["result"].get("required", [])
    base_schema["unevaluatedProperties"] = False

    return {**base_schema, "components": {"schemas": schemas}}


@lru_cache
def _request_schemas_for_method(name: str) -> OrderedDictType[str, Any]:
    """
    Return a dict of schemas for all parameters with correct structure for jsonschema validation.

    {
        "param1": {
            // schema 1
            "components": {
                "schemas": {
                    // rest of the schemas
                    // ...
                }
            }
        },
        "param2: {
            // schema 2
            "components": {
                "schemas": {
                    // rest of the schemas
                    // ...
                }
            }
        },
        // ...
    }

    See _response_schema_for_method docstring for more detailed explanation.
    """
    methods, schemas = _load_schemas()
    params_json: List[Dict[str, Any]] = methods[name]["params"]

    request_schemas = OrderedDict()
    for param in params_json:
        name = param["name"]
        is_required = param.get("required", False)
        schema = {
            **param["schema"],
            "is_required": is_required,
            "components": {"schemas": schemas},
        }
        request_schemas[name] = schema

    return request_schemas


def _assert_valid_rpc_schema(data: Dict[str, Any], method_name: str):
    """
    Check if rpc response is valid against the schema for given method name
    """
    schema = _response_schema_for_method("starknet_" + method_name)
    validate(data, schema=schema)


def _assert_valid_rpc_request(*args, method_name: str, **kwargs):
    """
    Validate if RPC request (parameters) is correct.

    Raise ValidationError if not.
    """
    schemas = _request_schemas_for_method("starknet_" + method_name)

    if args and kwargs:
        raise ValueError("Cannot validate schemas with both args and kwargs provided.")

    if args:
        if len(args) > len(schemas):
            raise ValidationError("Too many arguments provided.")

        for name, arg in zip_longest(schemas.keys(), args, fillvalue="missing"):
            if arg == "missing":
                raise ValidationError(f"""Missing positional argument \"{name}\".""")

            validate(arg, schemas[name])
        return

    if kwargs:
        if len(kwargs) > len(schemas):
            raise ValidationError("Too many arguments provided.")

        for name, schema in schemas.items():
            if name not in kwargs:
                if schema["is_required"]:
                    raise ValidationError(f"""Missing keyword argument \"{name}\".""")
                continue

            value = kwargs[name]
            validate(value, schema)
        return

    if len(schemas) != 0:
        raise ValidationError(
            f"0 arguments provided to function expecting {len(schemas)} arguments."
        )


class ParamsValidationErrorWrapper(Exception):
    """
    Wrapper for ValidationError raised during request validation
    """

    def __init__(self, err: ValidationError):
        super().__init__("Failed to validate schema for params.")
        self.validation_error = err

    def __str__(self):
        return (
            f"""Got invalid value for parameter: \"{self.validation_error.message}\""""
        )


class ResponseValidationErrorWrapper(Exception):
    """
    Wrapper for ValidationError raised during response validation
    """

    def __init__(self, err: ValidationError):
        super().__init__("Failed to validate schema for response.")
        self.validation_error = err

    def __str__(self):
        return f"""Devnet tried to return invalid value: \"{self.validation_error.message}\""""


def validate_schema(method_name: str):
    """
    Decorator ensuring that call to rpc method and its response are valid
    in respect to RPC specification schemas.
    """

    def decorator(func):
        @wraps(func)
        async def wrapper(*args, **kwargs):
            config = state.starknet_wrapper.config

            if config.validate_rpc_requests:
                try:
                    _assert_valid_rpc_request(*args, **kwargs, method_name=method_name)
                except ValidationError as err:
                    raise ParamsValidationErrorWrapper(err) from err

            result = await func(*args, **kwargs)

            if config.validate_rpc_responses:
                try:
                    _assert_valid_rpc_schema(result, method_name)
                except ValidationError as err:
                    raise ResponseValidationErrorWrapper(err) from err

            return result

        return wrapper

    return decorator
