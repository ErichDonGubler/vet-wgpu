#!/usr/bin/env bash

set -eu

jq --slurpfile pulls pulls.json \
   --slurpfile reviews reviews.json \
   --argjson trusted '{ "jimblandy": true, "kvark": true, "nical": true }' \
   --raw-output \
   '{
        commit_sha,
        author: .user.login,
        approvers: [ $reviews[0][.number | tostring].reviews[] | select(.state == "APPROVED") | .user.login] | unique,
        merger: $pulls[0][.number | tostring].merged_by.login
   }
   | ( .vetted =
           (.author | in($trusted))
           or (.merger | in($trusted))
           or (.approvers | map(in($trusted)) | any)
     )
   | ( .status = if .vetted then "vetted" else "needs vet" end )
   | "\(.commit_sha)\t\(.author)\t\(.approvers | join(","))\t\(.merger)\t\(.status)"

' \
   commits.json
