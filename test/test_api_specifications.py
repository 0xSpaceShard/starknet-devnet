"""
Test /api endpoint.
"""

from starknet_devnet.server import app

from .settings import APP_URL


def test_api_endpoint():
    """Assert that /api endpoint return list of endpoints"""
    response = app.test_client().get(f"{APP_URL}/api")
    assert response.status_code == 200
    assert response.json["/api"]["functionName"] == "api"
    assert response.json["/api"]["doc"] == "Return available endpoints."
