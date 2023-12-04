#!/bin/bash

set -eu

curl -L https://foundry.paradigm.xyz | bash
export PATH="$PATH:/home/fabijanc/.foundry/bin"

# TODO if necessary, add the above PATH modification to BASH_ENV on circleci

foundryup
anvil --version
