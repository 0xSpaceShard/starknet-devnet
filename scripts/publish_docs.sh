#!/bin/bash

set -euo pipefail
cd website && npm ci
USE_SSH="true" npm run deploy
