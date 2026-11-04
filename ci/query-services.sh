#!/usr/bin/env bash
set -euo pipefail

cleanup_l2() {
  local catalog_dir base
  catalog_dir=$(docker inspect --format '{{index .Config.Labels "ontoforge.l2.catalog-dir"}}' ontoforge-trino-l2 2>/dev/null || true)
  docker rm -f ontoforge-trino-l2 ontoforge-trino-l2-loose >/dev/null 2>&1 || true
  base=$(realpath -m "${RUNNER_TEMP:-${TMPDIR:-/tmp}}")
  if [[ -n "$catalog_dir" && "$(dirname "$(realpath -m "$catalog_dir")")" == "$base" && "$(basename "$catalog_dir")" == trino-l2-catalog.* ]]; then
    rm -rf -- "$catalog_dir"
  fi
}

case "${1:-}" in
  postgres)
    password=$(openssl rand -hex 24)
    if [[ "${GITHUB_ACTIONS:-}" == true ]]; then printf '::add-mask::%s\n' "$password"; fi

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
    printf 'ONTOFORGE_PG_URL=postgres://postgres:%s@127.0.0.1:5432/editstore\n' "$password" >> "$GITHUB_ENV"

    docker run -d --name postgres \
      -e POSTGRES_PASSWORD="$password" \
      -p 127.0.0.1:5432:5432 \
      --health-cmd 'pg_isready -U postgres' \
      --health-interval 5s --health-timeout 5s --health-retries 20 \
      postgres:17.11@sha256:2d2b8998d31037bf721cfdf764d76ba74171b4fab3431b7f72c27c56ddbdf9e3

    for attempt in {1..60}; do
      if docker exec postgres psql -U postgres -d postgres -Atqc 'SELECT 1' >/dev/null 2>&1; then
        exit 0
      fi
      sleep 2
    done
    echo '::error::PostgreSQL did not accept connections within 120 s'
    exit 1
    ;;
  seed)
    root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
    fixtures="$root/examples/ontologies/aviation/bindings/dev/fixtures"
    if command -v cygpath >/dev/null 2>&1; then fixtures=$(cygpath -w "$fixtures"); fi
    MSYS_NO_PATHCONV=1 docker cp "$fixtures" postgres:/tmp/ontoforge-fixtures
    docker exec postgres psql -U postgres -v ON_ERROR_STOP=1 -d postgres -c 'CREATE DATABASE demo' -c 'CREATE DATABASE editstore'
    MSYS_NO_PATHCONV=1 docker exec -i postgres psql -U postgres -v ON_ERROR_STOP=1 -v fixtures=/tmp/ontoforge-fixtures -d demo < "$root/ci/pg/seed.sql"
    docker exec -i postgres psql -U postgres -v ON_ERROR_STOP=1 -d demo < "$root/ci/pg/rowpred_corpus.sql"
    docker exec -i postgres psql -U postgres -v ON_ERROR_STOP=1 -d demo < "$root/ci/pg/query_corpus.sql"
    ;;
  trino)
    : "${TRINO_CATALOG_DIR:?PostgreSQL setup must run first}"
    mount_catalog=$TRINO_CATALOG_DIR
    network=(--network host)
    if command -v cygpath >/dev/null 2>&1; then
      sed -i 's@jdbc:postgresql://localhost:@jdbc:postgresql://host.docker.internal:@' "$TRINO_CATALOG_DIR"/*.properties
      mount_catalog=$(cygpath -w "$TRINO_CATALOG_DIR")
      network=(-p 127.0.0.1:8080:8080)
    fi
    MSYS_NO_PATHCONV=1 docker run -d --name trino "${network[@]}" \
      -v "$mount_catalog:/etc/trino/catalog:ro" \
      trinodb/trino:483@sha256:db58cc93e593a2706553745f276bb119c9810e69918be56ecde088ba7ccb0534
    ;;
  trino-l2)
    root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
    catalog_dir=$(mktemp -d "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/trino-l2-catalog.XXXXXXXX")
    for catalog in demo outside editstore; do
      printf 'connector.name=tpch\n' > "$catalog_dir/$catalog.properties"
    done
    printf 'connector.name=memory\n' > "$catalog_dir/hive.properties"
    chmod 755 "$catalog_dir"
    chmod 644 "$catalog_dir"/*.properties
    node -e 'const fs=require("fs"); const r=JSON.parse(fs.readFileSync(process.argv[1])); r.catalogs.unshift({user:"l2",catalog:"outside",allow:"read-only"}); r.tables.unshift({user:"l2",catalog:"outside",schema:"tiny",table:"nation",privileges:["SELECT"]}); fs.writeFileSync(process.argv[2],JSON.stringify(r));' "$root/ci/trino/l2/rules.json" "$catalog_dir/loose-rules.json"
    chmod 644 "$catalog_dir/loose-rules.json"
    mount_root=$root
    mount_catalog=$catalog_dir
    if command -v cygpath >/dev/null 2>&1; then
      mount_root=$(cygpath -w "$root")
      mount_catalog=$(cygpath -w "$catalog_dir")
    fi
    MSYS_NO_PATHCONV=1 docker run -d --name ontoforge-trino-l2 \
      --label "ontoforge.l2.catalog-dir=$catalog_dir" \
      -p 127.0.0.1:58081:8080 \
      -v "$mount_catalog:/etc/trino/catalog:ro" \
      -v "$mount_root/ci/trino/l2/config.properties:/etc/trino/config.properties:ro" \
      -v "$mount_root/ci/trino/l2/access-control.properties:/etc/trino/access-control.properties:ro" \
      -v "$mount_root/ci/trino/l2/rules.json:/etc/trino/l2-rules.json:ro" \
      trinodb/trino:483@sha256:db58cc93e593a2706553745f276bb119c9810e69918be56ecde088ba7ccb0534
    for attempt in {1..60}; do
      if curl -fsS --max-time 2 http://127.0.0.1:58081/v1/info 2>/dev/null | node -e 'let s="";process.stdin.on("data",c=>s+=c).on("end",()=>{try{const d=JSON.parse(s);process.exit(d.nodeVersion?.version==="483"&&d.starting===false?0:1)}catch{process.exit(1)}})' 2>/dev/null; then
        if ! docker exec ontoforge-trino-l2 sh -c 'test -f /etc/trino/l2-rules.json && grep -q "access-control.name=file" /etc/trino/access-control.properties'; then
          cleanup_l2
          echo 'Trino L2 ACL files are not mounted' >&2
          exit 1
        fi
        for catalog in demo outside editstore; do
          if ! docker exec ontoforge-trino-l2 trino --user ontology --execute "SELECT 1 FROM $catalog.tiny.nation LIMIT 1" >/dev/null; then
            cleanup_l2
            echo "Trino L2 fixture $catalog.tiny.nation is unavailable" >&2
            exit 1
          fi
        done
        if ! MSYS_NO_PATHCONV=1 docker exec -i ontoforge-trino-l2 trino --user ontology --file /dev/stdin < "$root/ci/trino/l2/seed.sql" >/dev/null; then
          cleanup_l2
          echo 'Trino L2 hive.sales fixture seed failed' >&2
          exit 1
        fi
        MSYS_NO_PATHCONV=1 docker run -d --name ontoforge-trino-l2-loose \
          -p 127.0.0.1:58082:8080 \
          -v "$mount_catalog:/etc/trino/catalog:ro" \
          -v "$mount_root/ci/trino/l2/config.properties:/etc/trino/config.properties:ro" \
          -v "$mount_root/ci/trino/l2/access-control.properties:/etc/trino/access-control.properties:ro" \
          -v "$mount_catalog/loose-rules.json:/etc/trino/l2-rules.json:ro" \
          trinodb/trino:483@sha256:db58cc93e593a2706553745f276bb119c9810e69918be56ecde088ba7ccb0534
        for control_attempt in {1..60}; do
          if curl -fsS --max-time 2 http://127.0.0.1:58082/v1/info 2>/dev/null | node -e 'let s="";process.stdin.on("data",c=>s+=c).on("end",()=>{try{const d=JSON.parse(s);process.exit(d.nodeVersion?.version==="483"&&d.starting===false?0:1)}catch{process.exit(1)}})' 2>/dev/null; then
            if ! MSYS_NO_PATHCONV=1 docker exec -i ontoforge-trino-l2-loose trino --user ontology --file /dev/stdin < "$root/ci/trino/l2/seed.sql" >/dev/null; then
              cleanup_l2
              echo 'Trino L2 loose hive.sales fixture seed failed' >&2
              exit 1
            fi
            exit 0
          fi
          sleep 2
        done
        cleanup_l2
        echo 'Trino L2 loose ACL control did not become ready' >&2
        exit 1
      fi
      sleep 2
    done
    docker logs --tail 40 ontoforge-trino-l2 >&2 || true
    cleanup_l2
    echo 'Trino L2 did not become ready as version 483 within 120 s' >&2
    exit 1
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
    cleanup_l2
    docker rm -f trino postgres >/dev/null 2>&1 || true
    if [[ -n "${TRINO_CATALOG_DIR:-}" && "$TRINO_CATALOG_DIR" == "$RUNNER_TEMP"/trino-catalog.* ]]; then
      rm -rf -- "$TRINO_CATALOG_DIR"
    fi
    ;;
  *)
    echo 'usage: query-services.sh {postgres|seed|trino|trino-l2|check-artifacts|cleanup}' >&2
    exit 2
    ;;
esac
