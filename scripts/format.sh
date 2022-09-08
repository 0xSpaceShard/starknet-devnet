#!/bin/bash

set -e

poetry run black $(git ls-files '*.py')
