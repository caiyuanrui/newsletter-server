#!/bin/bash

set -x
set -eo pipefail

sqlx prepare || exit 1

docker build -t zero2prod -f Dockerfile .
