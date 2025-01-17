#!/bin/bash

set -x
set -eo pipefail


docker build -t zero2prod -f Dockerfile .
