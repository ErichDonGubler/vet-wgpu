#!/usr/bin/env bash

set -eu

jq --slurpfile pulls pulls.json \
   --slurpfile reviews reviews.json \
   --raw-output \
   '{
        commit_sha,
        author: .user.login,
        approvers: [ $reviews[0][.number | tostring].reviews[] | select(.state == "APPROVED") | .user.login] | unique,
        merger: $pulls[0][.number | tostring].merged_by.login
   }
   | "\(.commit_sha)\t\(.author)\t\(.approvers | join(","))\t\(.merger)"

' \
   commits.json
