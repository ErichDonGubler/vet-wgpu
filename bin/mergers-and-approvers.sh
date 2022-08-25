#!/usr/bin/env bash

set -eu

jq --slurpfile sha_to_pr sha-to-pr \
   --slurpfile pulls pulls.json \
   --slurpfile pr_to_approvers pr-to-approvers \
   '$sha_to_pr[0][.commit_sha]. as $pull
   '$sha_to_pr[0][.] as $sha_with_pr |
      {
          commit: .,
          author: $sha_with_pr.author,
          approvers: $pr_to_approvers[0][$sha_with_pr.pull_request | tostring].approvers
      }' \
   commits.json
