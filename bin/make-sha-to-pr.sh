#!/usr/bin/env bash

set -eu

(
    cd commits
    for commit in *; do
        jq '.[0] | {
            commit_sha: $commit,
            pull_request: .number
        }' $commit
    done
) \
    | jq --slurp 'INDEX(.commit)' > sha-to-pr.tmp

mv sha-to-pr.tmp sha-to-pr
