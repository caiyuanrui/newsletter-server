#!/bin/bash

set -x
set -eo pipefail

pids=$(docker ps -q -a)

if [ -n "$pids" ]; then
  docker stop $pids
fi

docker container prune -f
docker image prune -f
