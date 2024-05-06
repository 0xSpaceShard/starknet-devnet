#!/usr/bin/env python

"""
This program tests if a command performs faster using independent t-test. At the top of the file,
there are command placeholders which you need to define. You may change other constants
if needed. Optionally, modify `performance_program`, a function used to simulate the
work that is timed.

The program will start the command defined in `ORIGINAL_COMMAND`,
run `performance_program` `SAMPLE_SIZE` times, store these times,
and repeat the same process for `IMPROVED_COMMAND`.
The times shall than be statistically tested and the result printed.
"""

import subprocess
import time
from typing import List

import requests
from scipy.stats import ttest_ind

DEVNET_PORT = "5050"
DEVNET_URL = f"http://localhost:{DEVNET_PORT}"
REQUEST_TIMEOUT = 2

ORIGINAL_COMMAND: str = ...
"""
The original baseline command used for starting Devnet. E.g.:
```
f"cargo run --release -- --port {DEVNET_PORT}"
```
Be sure to have compiled the program before executing the script to avoid timeout.
"""

IMPROVED_COMMAND: str = ...
"""
The command used for starting Devnet in improved mode. E.g.:
```
f"cargo run --release -- --port {DEVNET_PORT} --lite-mode"
```
"""

ALTERNATIVE_HYPOTHESIS = "greater"
"""
The null-hypothesis is that the two analyzed samples come from equal sources,
i.e. that the two commands under test perform equally fast. The alternative is "greater"
because the original command is supposed to be slower, i.e. yield greater times.
If you want to use this script to test if two commands are different,
change the alternative to "two-sided".
"""

SAMPLE_SIZE = 20


def ensure_process_started(proc: subprocess.Popen):
    """Ensure the process under test is started"""
    max_retries = 20
    for _ in range(max_retries):
        if proc.returncode is not None:
            raise RuntimeError(f"Process exited with returncode {proc.returncode}")

        try:
            resp = requests.get(f"{DEVNET_URL}/is_alive", timeout=REQUEST_TIMEOUT)
            if resp.status_code == 200:
                return
        except requests.exceptions.ConnectionError:
            pass

        time.sleep(0.5)

    raise RuntimeError("Could not start process")


def performance_program():
    """
    The program whose performance time is measured for sample generation.
    You may completely change the execution logic.
    """
    mint_url = f"{DEVNET_URL}/mint"
    req_body = {"amount": 1, "address": "0x1"}
    for _ in range(500):
        resp = requests.post(mint_url, json=req_body, timeout=REQUEST_TIMEOUT)
        assert resp.status_code == 200


def terminate_and_wait(proc: subprocess.Popen):
    """Terminates the process and waits."""
    proc.terminate()
    proc.wait()


def get_sample(command: str, size: int) -> List[float]:
    """
    Run `command` and run `performance_program` `size` times.
    Returns a list containing `size` measured times.
    """
    total_start_time = time.time()

    times = []

    for _ in range(size):
        with subprocess.Popen(
            command.split(), stdout=subprocess.DEVNULL
        ) as command_proc:
            ensure_process_started(command_proc)

            start_time = time.time()
            performance_program()
            measured_time = time.time() - start_time

            print(f"Measured time: {measured_time}")
            times.append(measured_time)

            terminate_and_wait(command_proc)

    total_time = time.time() - total_start_time
    print(f"Collected sample of size {size} in {total_time:.2f}s")
    return times


def main():
    """Run statistical testing"""

    samples = []
    for command in [ORIGINAL_COMMAND, IMPROVED_COMMAND]:
        print(f"Collecting sample for: {command}")
        samples.append(get_sample(command, SAMPLE_SIZE))

    result = ttest_ind(samples[0], samples[1], alternative=ALTERNATIVE_HYPOTHESIS)
    print(result)


if __name__ == "__main__":
    main()
