#!/usr/bin/env bash

set -eu

jq --slurpfile pulls pulls.json \
   --slurpfile reviews reviews.json \
   '{
        commit_sha,
        author: .user.login,
        approvers: [ $reviews[0][.number | tostring].reviews[] | select(.state == "APPROVED") | .user.login ],
        merger: $pulls[0][.number | tostring].merged_by.login
   }' \
   commits.json
