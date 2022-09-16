#!/usr/bin/env bash

set -eu

if ! [ -f commit-list ]; then
    echo "Run this script in the directory containing the 'commit-list' file." >&2
    exit 1
fi

rm -rf commits commit-pulls pulls reviews
rm -f commits.json commit-pulls.json pulls.json reviews.json mergers-and-approvers.tsv
