"""Constants used in test files."""

import socket


def bind_free_port(host):
    """return assigned free port and test base endpoint"""
    sock = socket.socket()
    sock.bind(("", 0))
    port = str(sock.getsockname()[1])
    return port, f"http://{host}:{port}"


HOST = "127.0.0.1"
PORT, APP_URL = bind_free_port(HOST)

L1_HOST = "localhost"
L1_PORT, L1_URL = bind_free_port(L1_HOST)
