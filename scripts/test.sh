#!/bin/bash
set -e

CMD="poetry run pytest -n auto --dist loadscope -v ${1:-test/}"
echo $CMD
$CMD
