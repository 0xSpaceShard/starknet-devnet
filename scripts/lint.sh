#!/bin/bash

set -e

poetry run pylint --load-plugins pylint_quotes $(git ls-files '*.py')
