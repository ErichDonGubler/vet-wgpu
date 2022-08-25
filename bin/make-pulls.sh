#!/usr/bin/env bash

set -eu

cd pulls

jq --slurp 'INDEX(.number)' > ../pulls.json *

