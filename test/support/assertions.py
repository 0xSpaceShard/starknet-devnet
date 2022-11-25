"""Utilities for schema validation"""

import json
from os.path import dirname, join

from jsonschema import RefResolver, validate


def assert_valid_schema(data, schema_file):
    """Checks whether the given data matches the schema"""

    schema = _load_json_schema(schema_file)
    schema_dir = join(dirname(__file__), "schemas")
    resolver = RefResolver(base_uri="file://" + schema_dir + "/", referrer=schema)
    return validate(data, schema, resolver=resolver)


def _load_json_schema(filename):
    """Loads the given schema file"""

    relative_path = join("schemas", filename)
    absolute_path = join(dirname(__file__), relative_path)

    with open(absolute_path, encoding="utf-8") as schema_file:
        return json.loads(schema_file.read())
