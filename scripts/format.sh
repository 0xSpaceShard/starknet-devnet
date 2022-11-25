#!/bin/bash

set -e

poetry run isort $(git ls-files '*.py')

poetry run black $(git ls-files '*.py')
