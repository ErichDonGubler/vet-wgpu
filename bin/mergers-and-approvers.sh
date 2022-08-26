#!/usr/bin/env bash

set -eu

jq --slurpfile pulls pulls.json \
   --slurpfile reviews reviews.json \
   --argjson trusted '{ "jimblandy": true, "kvark": true, "nical": true }' \
   --raw-output \
   --slurp \
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
   commits.json
