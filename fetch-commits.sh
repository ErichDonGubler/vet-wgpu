#!/usr/bin/env bash

set -eu

if ! [ -f commit-list ]; then
    echo "Run this script in the directory containing the `commit-list` file." >&2
    exit 1
fi

source ./repo.sh

rm -f commits/* commit-pulls/*
mkdir -p commits commit-pulls

# Fetch the pull requests associated with each commit.
for commit in $(cat commit-list); do
    echo commit $commit
    gh api "/repos/$owner/$repo/commits/$commit" | jq . > commits/$commit
    echo pulls for $commit
    gh api "/repos/$owner/$repo/commits/$commit/pulls" | jq . > commit-pulls/$commit
    sleep 0.1
done
