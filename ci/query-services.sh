#!/usr/bin/env bash
set -euo pipefail

case "${1:-}" in
  postgres)
    password=$(openssl rand -hex 24)
    printf '::add-mask::%s\n' "$password"

    catalog_dir=$(mktemp -d "$RUNNER_TEMP/trino-catalog.XXXXXXXX")
    printf 'TRINO_CATALOG_DIR=%s\n' "$catalog_dir" >> "$GITHUB_ENV"
    cp -a ci/trino/catalog/. "$catalog_dir/"
    chmod 755 "$catalog_dir"
    for catalog in demo editstore; do
      file="$catalog_dir/$catalog.properties"
      grep -q '^connection-password=' "$file"
      sed -i "s/^connection-password=.*/connection-password=$password/" "$file"
      chmod 644 "$file"
    done

    printf 'PGPASSWORD=%s\n' "$password" >> "$GITHUB_ENV"
    printf 'ONTOFORGE_PG_URL=postgres://postgres:%s@localhost:5432/editstore\n' "$password" >> "$GITHUB_ENV"

    docker run -d --name postgres \
      -e POSTGRES_PASSWORD="$password" \
      -p 127.0.0.1:5432:5432 \
      --health-cmd 'pg_isready -U postgres' \
      --health-interval 5s --health-timeout 5s --health-retries 20 \
      postgres:17.11@sha256:2d2b8998d31037bf721cfdf764d76ba74171b4fab3431b7f72c27c56ddbdf9e3

    for attempt in {1..60}; do
      if PGPASSWORD="$password" psql -h localhost -U postgres -d postgres -Atqc 'SELECT 1' >/dev/null 2>&1; then
        exit 0
      fi
      sleep 2
    done
    echo '::error::PostgreSQL did not accept connections within 120 s'
    exit 1
    ;;
  trino)
    : "${TRINO_CATALOG_DIR:?PostgreSQL setup must run first}"
    docker run -d --name trino --network host \
      -v "$TRINO_CATALOG_DIR:/etc/trino/catalog:ro" \
      trinodb/trino:483@sha256:db58cc93e593a2706553745f276bb119c9810e69918be56ecde088ba7ccb0534
    ;;
  check-artifacts)
    : "${PGPASSWORD:?PostgreSQL setup must run first}"
    shift
    for directory in "$@"; do
      if [[ -d "$directory" ]]; then
        status=0
        grep -rFq -- "$PGPASSWORD" "$directory" || status=$?
        if [[ "$status" -eq 0 ]]; then
          echo "::error::Job credential found in artifact directory: $directory"
          exit 1
        elif [[ "$status" -ne 1 ]]; then
          echo "::error::Could not scan artifact directory: $directory"
          exit 1
        fi
      fi
    done
    ;;
  cleanup)
    docker rm -f trino postgres >/dev/null 2>&1 || true
    if [[ -n "${TRINO_CATALOG_DIR:-}" && "$TRINO_CATALOG_DIR" == "$RUNNER_TEMP"/trino-catalog.* ]]; then
      rm -rf -- "$TRINO_CATALOG_DIR"
    fi
    ;;
  *)
    echo 'usage: query-services.sh {postgres|trino|check-artifacts|cleanup}' >&2
    exit 2
    ;;
esac
