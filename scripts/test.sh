#!/bin/bash
set -eu

./scripts/check_starknet_artifacts_version.sh

CMD="poetry run pytest -n auto --dist loadscope -v ${1:-test/}"
echo $CMD
$CMD
