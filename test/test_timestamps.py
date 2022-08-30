"""
Test block timestamps
"""

import math
import time
import pytest
import requests

from starknet_devnet.block_info_generator import BlockInfoGenerator, BlockInfo
from starknet_devnet.general_config import DEFAULT_GENERAL_CONFIG

from .shared import ARTIFACTS_PATH
from .util import devnet_in_background, deploy, call, get_block
from .settings import APP_URL

TS_CONTRACT_PATH = f"{ARTIFACTS_PATH}/timestamp.cairo/timestamp.json"
TS_ABI_PATH = f"{ARTIFACTS_PATH}/timestamp.cairo/timestamp_abi.json"

SET_TIME_ARGUMENT = 1514764800


def deploy_ts_contract():
    """Deploys the timestamp contract"""
    return deploy(TS_CONTRACT_PATH)


def get_ts_from_contract(address):
    """Returns the timestamp of the contract"""
    return int(
        call(
            function="get_timestamp",
            address=address,
            abi_path=TS_ABI_PATH,
        )
    )


def get_ts_from_last_block():
    """Returns the timestamp of the last block"""
    return get_block(parse=True)["timestamp"]


def increase_time(time_s):
    """Increases the block timestamp offset"""
    increase_time_response = requests.post(
        f"{APP_URL}/increase_time", json={"time": time_s}
    )

    if increase_time_response.status_code == 200:
        assert increase_time_response.json().get("timestamp_increased_by") == time_s

    return increase_time_response


def set_time(time_s):
    """Sets the block timestamp and offset"""
    set_time_response = requests.post(f"{APP_URL}/set_time", json={"time": time_s})

    if set_time_response == 200:
        assert set_time_response.json().get("next_block_timestamp") == time_s

    return set_time_response


@pytest.mark.timestamps
@devnet_in_background()
def test_timestamps():
    """Test timestamp"""
    deploy_info = deploy_ts_contract()
    ts_after_deploy = get_ts_from_last_block()

    ts_from_first_call = get_ts_from_contract(deploy_info["address"])

    assert ts_after_deploy == ts_from_first_call

    # deploy another contract to generate a new block
    deploy_ts_contract()
    ts_after_second_deploy = get_ts_from_last_block()

    assert ts_after_second_deploy > ts_from_first_call

    ts_from_second_call = get_ts_from_contract(deploy_info["address"])

    assert ts_after_second_deploy == ts_from_second_call
    assert ts_from_second_call > ts_from_first_call


@pytest.mark.timestamps
@devnet_in_background()
def test_increase_time():
    """Test timestamp increase time"""
    start = time.time()
    deploy_info = deploy_ts_contract()
    ts_after_deploy = get_ts_from_last_block()

    first_block_ts = get_ts_from_contract(deploy_info["address"])

    assert ts_after_deploy == first_block_ts

    # increase time by 1 day
    increase_time(86400)

    # deploy another contract to generate a new block
    deploy_ts_contract()

    second_block_ts = get_ts_from_last_block()

    assert second_block_ts - first_block_ts >= 86400
    elapsed_time = math.ceil(time.time() - start)
    assert second_block_ts < first_block_ts + 86400 + elapsed_time


@pytest.mark.timestamps
@devnet_in_background()
def test_set_time():
    """Test timestamp set time"""
    deploy_info = deploy_ts_contract()
    first_block_ts = get_ts_from_last_block()

    ts_from_first_call = get_ts_from_contract(deploy_info["address"])

    assert first_block_ts == ts_from_first_call

    # set time to 1 day after the deploy
    set_time(first_block_ts + 86400)

    ts_after_set = get_ts_from_last_block()

    assert ts_after_set == first_block_ts

    # generate a new block by deploying a new contract
    deploy_ts_contract()

    second_block_ts = get_ts_from_last_block()

    assert second_block_ts == first_block_ts + 86400

    # generate a new block by deploying a new contract
    deploy_ts_contract()

    third_block_ts = get_ts_from_last_block()

    # check if offset is still the same
    assert third_block_ts - first_block_ts >= 86400


@pytest.mark.timestamps
@devnet_in_background("--start-time", str(SET_TIME_ARGUMENT))
def test_set_time_argument():
    """Test timestamp set time argument"""
    first_block_ts = get_ts_from_last_block()

    assert first_block_ts == SET_TIME_ARGUMENT


@pytest.mark.timestamps
@devnet_in_background()
def test_set_time_errors():
    """Test timestamp set time negative"""
    deploy_ts_contract()

    response = set_time(-1)
    message = response.json()["message"]

    assert response.status_code == 400
    assert message == "time value must be greater than 0."

    response = set_time(None)
    message = response.json()["message"]

    assert response.status_code == 400
    assert message == "time value must be provided."

    response = set_time("not an int")
    message = response.json()["message"]

    assert response.status_code == 400
    assert message == "time value must be an integer."


@pytest.mark.timestamps
@devnet_in_background()
def test_increase_time_errors():
    """Test timestamp increase time negative"""
    deploy_ts_contract()

    response = increase_time(-1)
    message = response.json()["message"]

    assert response.status_code == 400
    assert message == "time value must be greater than 0."

    response = increase_time(None)
    message = response.json()["message"]

    assert response.status_code == 400
    assert message == "time value must be provided."

    response = increase_time("not an int")
    message = response.json()["message"]

    assert response.status_code == 400
    assert message == "time value must be an integer."


@pytest.mark.timestamps
def test_block_info_generator():
    """Test block info generator"""
    start = int(time.time())
    block_info = BlockInfo.create_for_testing(block_number=0, block_timestamp=start)

    # Test if start time is set by the constructor
    generator = BlockInfoGenerator(start_time=10)

    block_with_start_time = generator.next_block(
        block_info=block_info, general_config=DEFAULT_GENERAL_CONFIG
    )

    assert block_with_start_time.block_timestamp == 10

    # Check if set time can be incrased

    generator.increase_time(22)

    block_after_increase = generator.next_block(
        block_info=block_info, general_config=DEFAULT_GENERAL_CONFIG
    )

    assert block_after_increase.block_timestamp == 32

    # Test if start time can be set after increase

    generator = BlockInfoGenerator()
    generator.increase_time(1_000_000_000)

    block_with_increase_time = generator.next_block(
        block_info=block_info, general_config=DEFAULT_GENERAL_CONFIG
    )

    assert block_with_increase_time.block_timestamp >= 1_000_000_000 + int(time.time())

    generator.set_next_block_time(222)
    block_after_set_time = generator.next_block(
        block_info=block_info, general_config=DEFAULT_GENERAL_CONFIG
    )

    assert block_after_set_time.block_timestamp == 222


@pytest.mark.timestamps
@devnet_in_background("--lite-mode")
def test_lite_mode_compatibility():
    """Tests compatibility with lite mode"""

    deploy_info = deploy_ts_contract()

    set_time(100)

    # deploy another contract to generate a new block
    deploy_ts_contract()

    time_from_contract = get_ts_from_contract(address=deploy_info["address"])
    assert time_from_contract == 100
