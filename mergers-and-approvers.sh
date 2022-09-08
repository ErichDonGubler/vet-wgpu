#!/usr/bin/env bash

set -eu

if ! [ -f commit-list ]; then
    echo "Run this script in the directory containing the 'commit-list' file." >&2
    exit 1
fi

if ! [ -d commits -a -d pulls -a -d reviews ]; then
    echo "There should be 'commits', 'pulls', and 'reviews' directories." >&2
    echo "Did you run 'fetch-commits.sh'?" >&2
    exit 1
fi

if ! [ -f trusted.json ]; then
    echo "Trusted auditors file 'trusted.json' doesn't exist." >&2
    exit 1
fi

# These initial indexing steps could winnow out a lot of data, since
# the GitHub API results are pretty verbose, and the final step
# doesn't actually need most of that. But we are not really limited by
# performance, and it's nice to have all the data we could possibly
# want ready at hand in the final step.

# Map each commit to its list of pull requests.
( cd commits && jq '.[0] | ( .commit_sha=input_filename )' *) \
> commits.json

# Map each pull request's number to its data.
jq --slurp 'INDEX(.number)' pulls/* \
> pulls.json   

# Map each pull request number to an array of its reviews.
( cd reviews && jq '{ pull: input_filename, reviews: . }' * )   \
| jq --slurp 'INDEX(.pull)'                                     \
> reviews.json

# For each commit, determine:
# - the pull request by which it was merged
# - its author
# - reviewers who marked it as approved
# - who ultimately merged the PR
#
# From among those parties, extract the set of trusted auditors who
# played those roles, if any.
#
# Sort by pull request number, and print as tab-separated values
jq --slurpfile pulls pulls.json                 \
   --slurpfile reviews reviews.json             \
   --argjson trusted "$(cat trusted.json)"      \
   --raw-output                                 \
   --slurp                                      \
   'map(
        {
            pull: .number,
            commit_sha,
            author: .user.login,
            approvers: [ $reviews[0][.number | tostring].reviews[]
                         | select(.state == "APPROVED")
                         | .user.login
                       ]
                       | unique,
            merger: $pulls[0][.number | tostring].merged_by.login
        }
        | ( .vetters = ( [ .author, .merger, .approvers[] | select(in($trusted)) ] | unique) )
   )
   | sort_by(.pull)[]
   | "\(.pull)\t\(.commit_sha)\t\(.author)\t\(.approvers | join(","))\t\(.merger)\t\(.vetters | join(","))"
   ' \
   commits.json \
   > mergers-and-approvers.tsv

