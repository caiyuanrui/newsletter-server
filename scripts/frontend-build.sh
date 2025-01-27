#!/bin/bash

set -x
set -eo pipefail

FRONTEND_PATH="../Projects/zero2prod-frontend"

(cd $FRONTEND_PATH && npm run build)

rm -rf public/*
cp -rp $FRONTEND_PATH/dist/* public
