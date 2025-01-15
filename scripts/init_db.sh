#!/bin/bash
set -x
set -eo pipefail

if ! [ -x "$(command -v sqlx)" ]; then
  echo >&2 "Error: sqlx is not installed."
  echo >&2 "Use:"
  echo >&2 "    cargo install --version=0.5.7 sqlx-cli --no-default-features --features postgres"
  echo >&2 "to install it."
  exit 1
fi

DB_USER=zero2prod
DB_PASSWARD=12345678
DB_NAME=newsletter
DB_HOST=localhost
DB_PORT=3306

export DB_PASSWARD=${DB_PASSWARD}
export DATABASE_URL=mysql://${DB_USER}:${DB_PASSWARD}@${DB_HOST}:${DB_PORT}/${DB_NAME}

sqlx database create
sqlx migrate run

>&2 echo "MySQL has been migrated, ready to go!"
