#!/usr/bin/env python3
"""Randomized tester for the `proof` base64 pattern in Starknet OpenRPC spec.

It validates two directions:
1) Generate random strings that match the regex pattern, then decode to bytes and
   verify they can be interpreted as big-endian packed u32 values.
2) Generate random u32 arrays, encode to base64, and verify the encoded string
   matches the regex pattern.

Usage examples:
  python scripts/test_proof_base64_pattern.py
  python scripts/test_proof_base64_pattern.py --iterations 200000 --seed 42
"""

from __future__ import annotations

import argparse
import base64
import random
import re
import struct
import sys
from dataclasses import dataclass

try:
    import exrex
except ImportError as exc:  # pragma: no cover - runtime dependency check
    raise SystemExit(
        "Missing dependency 'exrex'. Install it with: pip install exrex"
    ) from exc

PATTERN_STR = r"^(?:[A-Za-z0-9+/]{16})*(?:[A-Za-z0-9+/]{5}[AQgw]==|[A-Za-z0-9+/]{10}[AEIMQUYcgkosw048]=)?$"
PATTERN = re.compile(PATTERN_STR)


@dataclass
class Failure:
    index: int
    value: str
    reason: str


def random_string_from_pattern(max_16_char_blocks: int) -> str:
    """Generate a random string from the proof regex using exrex."""
    candidate = exrex.getone(PATTERN_STR)
    if not PATTERN.fullmatch(candidate):
        raise AssertionError("Internal generator error: candidate did not match pattern")
    return candidate


def decode_as_u32_array(b64_text: str) -> tuple[bool, str]:
    """Return (ok, reason_if_not_ok)."""
    try:
        raw = base64.b64decode(b64_text, validate=True)
    except Exception as exc:  # noqa: BLE001
        return False, f"base64 decode failed: {exc}"

    if len(raw) % 4 != 0:
        return False, f"decoded length is {len(raw)} bytes (not divisible by 4)"

    # Ensure unpack is possible; this should always succeed if length % 4 == 0.
    if raw:
        _ = struct.unpack(f">{len(raw) // 4}I", raw)

    # Optional canonical check: if re-encoding differs, input is non-canonical.
    reencoded = base64.b64encode(raw).decode("ascii")
    if reencoded != b64_text:
        return False, "decodes but is not canonical base64 representation"

    return True, ""


def random_u32_array(rng: random.Random, max_u32_len: int) -> list[int]:
    length = rng.randint(0, max_u32_len)
    return [rng.getrandbits(32) for _ in range(length)]


def encode_u32_array(values: list[int]) -> str:
    if not values:
        return ""
    raw = struct.pack(f">{len(values)}I", *values)
    return base64.b64encode(raw).decode("ascii")


def test_pattern_to_decode(
    iterations: int,
    max_16_char_blocks: int,
    max_fail_examples: int,
) -> list[Failure]:
    failures: list[Failure] = []
    for i in range(iterations):
        s = random_string_from_pattern(max_16_char_blocks=max_16_char_blocks)
        ok, reason = decode_as_u32_array(s)
        if not ok and len(failures) < max_fail_examples:
            failures.append(Failure(index=i, value=s, reason=reason))
    return failures


def test_u32_to_pattern(
    rng: random.Random,
    iterations: int,
    max_u32_len: int,
    max_fail_examples: int,
) -> list[Failure]:
    failures: list[Failure] = []
    for i in range(iterations):
        arr = random_u32_array(rng, max_u32_len=max_u32_len)
        b64 = encode_u32_array(arr)
        if not PATTERN.fullmatch(b64) and len(failures) < max_fail_examples:
            failures.append(Failure(index=i, value=b64, reason="encoded value does not match regex"))
    return failures


def print_failures(title: str, failures: list[Failure]) -> None:
    print(f"\n{title}")
    if not failures:
        print("  none")
        return
    for failure in failures:
        print(f"  - idx={failure.index}: {failure.reason}")
        print(f"    value={failure.value}")


def main() -> int:
    parser = argparse.ArgumentParser(description="Randomized test for Starknet proof base64 regex")
    parser.add_argument("--iterations", type=int, default=100_000, help="iterations per direction")
    parser.add_argument(
        "--max-16-char-blocks",
        type=int,
        default=24,
        help="max number of 16-char regex blocks for exrex pattern generation",
    )
    parser.add_argument(
        "--max-u32-len",
        type=int,
        default=128,
        help="max u32 array length when generating random arrays",
    )
    parser.add_argument("--seed", type=int, default=None, help="random seed for reproducibility")
    parser.add_argument(
        "--max-fail-examples",
        type=int,
        default=10,
        help="max failures to record per direction",
    )
    args = parser.parse_args()

    random.seed(args.seed)
    rng = random.Random(args.seed)

    print("Testing pattern:")
    print(f"  {PATTERN_STR}")
    print(f"Iterations per direction: {args.iterations}")
    print(f"Seed: {args.seed}")

    failures_pattern_to_decode = test_pattern_to_decode(
        iterations=args.iterations,
        max_16_char_blocks=args.max_16_char_blocks,
        max_fail_examples=args.max_fail_examples,
    )

    failures_u32_to_pattern = test_u32_to_pattern(
        rng=rng,
        iterations=args.iterations,
        max_u32_len=args.max_u32_len,
        max_fail_examples=args.max_fail_examples,
    )

    print_failures("Direction 1: pattern -> decode -> u32[]", failures_pattern_to_decode)
    print_failures("Direction 2: random u32[] -> base64 -> pattern", failures_u32_to_pattern)

    has_failures = bool(failures_pattern_to_decode or failures_u32_to_pattern)
    if has_failures:
        print("\nResult: FAIL (counterexamples found)")
        return 1

    print("\nResult: PASS (no counterexamples found in sampled tests)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
