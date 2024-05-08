#!/usr/bin/env python

"""
This program tests if a command performs faster and with less memory. This is achieved
using independent t-test. At the top of the file, there are command placeholders which
you need to define. You may change other constants if needed. Optionally, modify
`performance_program`, a function used to simulate the work that is benchmarked.

The program will start the command defined in `ORIGINAL_COMMAND`,
run `performance_program` `SAMPLE_SIZE` times, store these measurements,
and repeat the same process for `IMPROVED_COMMAND`. Freely modify these three variables.
The measurements shall then be statistically tested and the results printed.
"""

import subprocess
import time
from typing import List, Tuple

import psutil
import requests
from scipy.stats import ttest_ind
from scipy.stats import describe

DEVNET_PORT = "5050"
DEVNET_URL = f"http://localhost:{DEVNET_PORT}"
REQUEST_TIMEOUT = 2

ORIGINAL_COMMAND: str = f"cargo run --release -- --port {DEVNET_PORT}"
"""
The original baseline command used for starting Devnet. Modify it freely.
Be sure to have compiled the program before executing the script to avoid timeout.
"""

IMPROVED_COMMAND: str = f"cargo run --release -- --port {DEVNET_PORT} --lite-mode"
"""
The command used for starting Devnet in improved mode. Modify it freely.
"""

ALTERNATIVE_HYPOTHESIS = "greater"
"""
The null-hypothesis is that the two analyzed samples come from equal sources,
i.e. that the two tested commands perform equally well. The alternative is "greater"
because the original command is supposed to be slower, i.e. yield greater times.
Or in terms of memory, that it uses more memory. If you want to use this script
to test if two commands are simply different, change the alternative to "two-sided".
"""

SAMPLE_SIZE = 2


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


def get_sample(command: str, size: int) -> Tuple[List[float], List[float]]:
    """
    Run `command` and run `performance_program` `size` times.
    Returns a tuple of:
      - a list containing `size` measured times in seconds
      - a list containing `size` measured memory usages in MB
    """
    total_start_time = time.time()

    time_measurements = []
    memory_measurements = []

    for _ in range(size):
        with subprocess.Popen(
            command.split(), stdout=subprocess.DEVNULL
        ) as command_proc:
            ensure_process_started(command_proc)

            command_proc_ps = psutil.Process(command_proc.pid)

            start_memory = command_proc_ps.memory_info()
            start_time = time.time()
            performance_program()
            measured_time = time.time() - start_time
            final_memory = command_proc_ps.memory_info()

            print(f"Measured time (s): {measured_time}")
            time_measurements.append(measured_time)

            measured_rss = (final_memory.rss - start_memory.rss) / 1e6
            print(f"Measured memory - rss (MB): {measured_rss}")
            memory_measurements.append(measured_rss)

            terminate_and_wait(command_proc)

    total_time = time.time() - total_start_time
    print(f"Collected samples in {total_time:.2f}s")
    print(f"\tTime sample:   {describe(time_measurements)}")
    print(f"\tMemory sample: {describe(memory_measurements)}")
    return time_measurements, memory_measurements


def main():
    """Run statistical testing"""

    time_samples = []
    memory_samples = []
    for command in [ORIGINAL_COMMAND, IMPROVED_COMMAND]:
        print(f"Collecting sample for: {command}")
        times, memories = get_sample(command, SAMPLE_SIZE)
        time_samples.append(times)
        memory_samples.append(memories)

    print("Statistical report:")
    time_result = ttest_ind(
        time_samples[0], time_samples[1], alternative=ALTERNATIVE_HYPOTHESIS
    )
    print("\tTime (s):  ", time_result)

    memory_result = ttest_ind(
        memory_samples[0], memory_samples[1], alternative=ALTERNATIVE_HYPOTHESIS
    )
    print("\tMemory (MB):", memory_result)


if __name__ == "__main__":
    main()
