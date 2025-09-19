"""
Measures the time needed to spawn Devnet. The program expects two CLI arguments: the path to an
executable binary and N, the number of iterations. The process is killed as soon as Devnet becomes
available via HTTP. The program prints the accumulated time of running Devnet N times.
"""

import subprocess
import sys
import time

import requests

DEVNET_URL = "http://127.0.0.1:5050"
REQUEST_TIMEOUT = 2


def ensure_process_started(proc: subprocess.Popen):
    """Ensure the process under test is started"""
    max_retries = 50
    for i in range(max_retries):
        if proc.returncode is not None:
            raise RuntimeError(f"Process exited with returncode {proc.returncode}")

        try:
            resp = requests.get(f"{DEVNET_URL}/is_alive", timeout=REQUEST_TIMEOUT)
            if resp.status_code == 200:
                print(f"DEBUG returning on i={i}")
                return
        except requests.exceptions.ConnectionError:
            pass

        time.sleep(0.1)

    raise RuntimeError("Could not start process")


def terminate_and_wait(proc: subprocess.Popen):
    """Terminates the process and waits."""
    proc.terminate()
    proc.wait()


def main():
    """Spawn Devnet"""
    command = sys.argv[1]
    iterations = int(sys.argv[2])

    start_time = time.time()
    for _ in range(iterations):
        with subprocess.Popen(
            command.split(), stdout=subprocess.DEVNULL
        ) as command_proc:
            ensure_process_started(command_proc)
            terminate_and_wait(command_proc)

    print("Time passed:", time.time() - start_time)


if __name__ == "__main__":
    main()
