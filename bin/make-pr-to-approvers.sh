#!/usr/bin/env bash

set -eu

(
    cd reviews
    for review in *; do
        jq --arg pull_request $review \
            '{
                 pull_request: $pull_request,
                 approvers: [ .[] | select( .state == "APPROVED" ) | .user.login ]
             }' \
             $review
    done
) \
    | jq --slurp 'INDEX(.pull_request)' > pr-to-approvers.tmp

mv pr-to-approvers.tmp pr-to-approvers
