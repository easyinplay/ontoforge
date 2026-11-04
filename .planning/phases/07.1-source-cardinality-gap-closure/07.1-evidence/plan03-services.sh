#!/usr/bin/env bash
set -euo pipefail
root=$(git rev-parse --show-toplevel)
cd "$root"
runtime="$root/target/phase071-cardinality-runtime"
evidence="$root/.planning/phases/07.1-source-cardinality-gap-closure/07.1-evidence"
export RUNNER_TEMP="$runtime"
export GITHUB_ENV="$runtime/service.env"
case "${1:-}" in
  start)
    for name in postgres trino ontoforge-trino-l2 ontoforge-trino-l2-loose; do
      if docker inspect "$name" >/dev/null 2>&1; then
        echo "Refusing existing container: $name" >&2
        exit 1
      fi
    done
    mkdir -p "$runtime"
    : > "$GITHUB_ENV"
    docker ps --format '{{.Names}} {{.Image}} {{.Status}}' > "$evidence/plan03-containers-before.log"
    bash ci/query-services.sh postgres
    set -a
    source "$GITHUB_ENV"
    set +a
    docker inspect --format '{{.Id}}' postgres > "$runtime/postgres.id"
    bash ci/query-services.sh seed
    docker exec postgres psql -U postgres -d demo -Atqc 'SELECT count(*) FROM aviation.cardinality_records' > "$evidence/plan03-seed-count.log"
    sha256sum ci/pg/seed.sql > "$evidence/plan03-seed.sha256"
    bash ci/query-services.sh trino
    docker inspect --format '{{.Id}}' trino > "$runtime/trino.id"
    for attempt in {1..60}; do
      if curl -fsS --max-time 2 http://localhost:8080/v1/info > "$runtime/info.json" 2>/dev/null; then
        if node -e 'const fs=require("fs"); const x=JSON.parse(fs.readFileSync(process.argv[1]));process.exit(x.nodeVersion?.version==="483"&&x.starting===false?0:1)' "$runtime/info.json"; then
          cp "$runtime/info.json" "$evidence/plan03-trino-info.json"
          docker exec trino trino --user ontoforge --execute 'SELECT tail_number, count(DISTINCT record_id) AS n FROM demo.aviation.cardinality_records WHERE tail_number IS NOT NULL GROUP BY tail_number ORDER BY tail_number' > "$evidence/plan03-real-seed-query.log"
          docker ps --format '{{.Names}} {{.Image}} {{.Status}}' > "$evidence/plan03-containers-running.log"
          exit 0
        fi
      fi
      sleep 2
    done
    echo 'Trino did not become ready' >&2
    exit 1
    ;;
  cleanup)
    set -a
    source "$GITHUB_ENV"
    set +a
    for name in postgres trino; do
      actual=$(docker inspect --format '{{.Id}}' "$name" 2>/dev/null || true)
      if [[ -n "$actual" && "$actual" != "$(cat "$runtime/$name.id")" ]]; then
        echo "Container ownership changed: $name" >&2
        exit 1
      fi
    done
    for name in ontoforge-trino-l2 ontoforge-trino-l2-loose; do
      if docker inspect "$name" >/dev/null 2>&1; then
        echo "Foreign L2 container present: $name" >&2
        exit 1
      fi
    done
    bash ci/query-services.sh check-artifacts "$evidence"
    bash ci/query-services.sh cleanup
    docker ps --format '{{.Names}} {{.Image}} {{.Status}}' > "$evidence/plan03-containers-after.log"
    echo 'Owned PostgreSQL and Trino cleaned; evidence credential scan passed.'
    ;;
  *) exit 2 ;;
esac
