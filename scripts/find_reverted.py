"""Find reverted txs in given range"""

import sys
import time

import requests


def extract_not_succeeded_from_block(block_number: int) -> list:
    """Check txs in a block"""
    while True:
        req = requests.get(
            f"https://external.integration.starknet.io/feeder_gateway/get_block?blockNumber={block_number}"
        )
        if req.status_code == 200:
            break

        print(f"Status code not OK; is: {req.status_code} ({req.text})")

        sleep_secs = 1
        print(f"Sleeping for {sleep_secs} s")
        time.sleep(sleep_secs)

    body = req.json()

    not_succeeded = []

    receipts = body["transaction_receipts"]
    for receipt in receipts:
        status = receipt["execution_status"]
        if status != "SUCCEEDED":
            print(f"{receipt['transaction_hash']} is {status}")
            not_succeeded.append(receipt["transaction_hash"])

    return not_succeeded


def main():
    """The main method"""
    try:
        from_block = int(sys.argv[1])
        to_block = int(sys.argv[2])
    except (IndexError, ValueError):
        sys.exit(f"{__file__}: <FROM_BLOCK> <TO_BLOCK>")

    print(f"Searching in [{from_block} to {to_block}]")

    not_succeeded = []
    for block_number in range(from_block, to_block + 1):
        not_succeeded.extend(extract_not_succeeded_from_block(block_number))
        time.sleep(0.2)

        if block_number % 100 == 0:
            print(f"At block {block_number}, found so far: {len(not_succeeded)}")

    print()
    print("Finished. Not succeeded txs:")
    print(*not_succeeded, sep="\n")
    print("Count:", len(not_succeeded))


if __name__ == "__main__":
    main()
