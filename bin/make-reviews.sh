#!/usr/bin/env bash

set -eu

(
    cd reviews
    jq '{ pull: input_filename, reviews: . }' *
) \
    | jq --slurp 'INDEX(.pull)' \
         > reviews.json


