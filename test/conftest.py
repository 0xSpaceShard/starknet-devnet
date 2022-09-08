"""
Fixtures for tests
"""

from __future__ import annotations

from test.util import run_devnet_in_background, terminate_and_wait

import pytest


@pytest.fixture(name="expected_tx_hash")
def fixture_expected_tx_hash(request):
    """
    Fixture to return values of expected tx hash
    """
    return request.param


@pytest.fixture(name="expected_block_hash")
def fixture_expected_block_hash(request):
    """
    Fixture to return values of expected block hash
    """
    return request.param


@pytest.fixture(name="run_devnet_in_background")
def fixture_run_devnet_in_background(request) -> None:
    """
    Run devnet instance in background
    """
    args = getattr(request, "param", [])
    proc = run_devnet_in_background(*args)
    try:
        yield
    finally:
        terminate_and_wait(proc)
