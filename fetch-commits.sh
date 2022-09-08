#!/usr/bin/env bash

set -eu

if ! [ -f commit-list ]; then
    echo "Run this script in the directory containing the `commit-list` file." >&2
    exit 1
fi

source ./config.sh

mkdir -p commits pulls reviews
rm -f commits/* pulls/* reviews/*

# Fetch the pull requests associated with each commit.
for commit in $(cat commit-list); do
    echo pulls for $commit
    gh api "/repos/$owner/$repo/commits/$commit/pulls" | jq . > commits/$commit
    sleep 0.1
done

# Extract PR numbers from each commit's list.
pulls=$(jq '.[].number' commits/* | sort -nu)

# Fetch data about each pull request, and the reviews associated with it.
for pull in $pulls; do
    echo pull $pull
    gh api "/repos/$owner/$repo/pulls/$pull" | jq . > pulls/$pull
    echo reviews for $pull
    gh api "/repos/$owner/$repo/pulls/$pull/reviews" | jq . > reviews/$pull
    sleep 0.1
done
