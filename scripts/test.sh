#!/bin/bash
set -eu

./scripts/check_starknet_artifacts_version.sh

# Using dist=loadfile because currently some tests might have certain collisions
# namely dump tests which do a clean up and potentially remove dumps created by other tests
CMD="poetry run pytest --numprocesses=auto --maxprocesses=8 --dist=loadfile -vv ${1:-test/}"
echo $CMD
$CMD
