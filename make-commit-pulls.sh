#!/usr/bin/env bash

set -eu

if ! [ -f commit-list ]; then
    echo "Run this script in the directory containing the 'commit-list' file." >&2
    exit 1
fi


# Isolate the commit -> pull request mapping, so that people can edit it.
(
    cd commit-pulls
    jq '{
        commit_sha: input_filename,
        pulls: [ .[] | .number ]
    }' * \
    | jq --slurp '
          reduce .[] as $commit (
              {};
              .[$commit.commit_sha] = { pulls: $commit.pulls }
          )'
) > commit-pulls.json

missing_pulls=$(jq --raw-output 'map_values(select(.pulls == [])) | keys | .[]' commit-pulls.json)
if [ -n "$missing_pulls" ]; then
    echo "Commits for which we found no pull requests:" >&2
    echo "$missing_pulls" >&2
fi

