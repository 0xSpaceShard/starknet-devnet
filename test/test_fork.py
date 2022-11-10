"""Test the forking feature"""

from .shared import CONTRACT_PATH
from .settings import bind_free_port, HOST
from .util import call, deploy, devnet_in_background

ORIGIN_PORT, ORIGIN_URL = bind_free_port(HOST)
FORK_PORT, FORK_URL = bind_free_port(HOST)


@devnet_in_background("--port", ORIGIN_PORT)
@devnet_in_background("--port", FORK_PORT)
def test_origin_not_changed_if_fork_changed():
    """Invoke on fork, assert origin unchanged"""

    deploy_info = deploy(CONTRACT_PATH, inputs=["0"], gateway_url=ORIGIN_URL)
    call()
