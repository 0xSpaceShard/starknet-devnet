#!/bin/bash

set -e

poetry run isort $(git ls-files '*.py') --check
