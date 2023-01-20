#!/usr/bin/env bash

set -eo pipefail

source .env

if [ -z $DATABASE_URL ]; then
  echo "DATABASE_URL must be set in .env file" >&2
  exit 1
fi

read -r DB_USER DB_PASSWORD DB_HOST DB_PORT DB_NAME <<< $( \
  echo $DATABASE_URL | \
  sed -E 's/postgres:\/\/(.+):(.+)@(.+):(.+)\/(.+)/\1 \2 \3 \4 \5/' \
)

docker run \
  -d \
  --name "${DB_NAME}_db" \
  -e POSTGRES_USER="$DB_USER" \
  -e POSTGRES_PASSWORD="$DB_PASSWORD" \
  -e POSTGRES_DB="$DB_NAME" \
  -p "$DB_PORT:5432" \
  postgres \
  -N 1000 \
  >/dev/null

sleep 1
until psql $DATABASE_URL >/dev/null -c '\q'; do
  echo "Failed to connect to the database. Retrying..." >&2
  sleep 1
done

echo "Database is running on $DB_HOST:$DB_PORT"
