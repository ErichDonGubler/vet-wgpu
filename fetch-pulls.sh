#!/usr/bin/env bash

set -eu

if ! [ -f commit-list ]; then
    echo "Run this script in the directory containing the `commit-list` file." >&2
    exit 1
fi

source ./repo.sh

rm -f pulls/* reviews/*
mkdir -p pulls reviews

# Extract PR numbers from each commit's list, and from the overrides.
pulls=$(
    (
        jq '.[].number' commit-pulls/*
        jq '.[] | .pulls[]' commit-pulls-overrides.json
    ) \
    | sort -nu
)

# Fetch data about each pull request, and the reviews associated with it.
for pull in $pulls; do
    echo pull $pull
    gh api "/repos/$owner/$repo/pulls/$pull" | jq . > pulls/$pull
    echo reviews for $pull
    gh api "/repos/$owner/$repo/pulls/$pull/reviews" | jq . > reviews/$pull
    sleep 0.1
done
