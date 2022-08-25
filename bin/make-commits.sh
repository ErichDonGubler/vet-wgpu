#!/usr/bin/env bash

set -eu

cd commits

jq '.[0] | ( .commit_sha=input_filename )' * > ../commits.json
