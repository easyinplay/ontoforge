#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
RUNNER_TEMP=${RUNNER_TEMP:-"$ROOT/target/phase7-services"}
ENV_FILE=${GITHUB_ENV:-"$RUNNER_TEMP/server-services.env"}
STATE_DIR="$RUNNER_TEMP/state"
SERVER_CONFIG="$STATE_DIR/ontoforge-server.toml"
SERVER_LOG="$STATE_DIR/server.log"
SERVER_PID_FILE="$STATE_DIR/server.pid"

mkdir -p "$RUNNER_TEMP" "$STATE_DIR"
log() { printf '[server-services] %s\n' "$*"; }
die() { printf '[server-services] ERROR: %s\n' "$*" >&2; exit 1; }

write_env() {
  local key value
  for key in TRINO_CATALOG_DIR PGPASSWORD ONTOFORGE_PG_URL ONTOFORGE_TRINO_URL ONTOFORGE_TRINO_L2_URL ONTOFORGE_TRINO_L2_LOOSE_URL ONTOFORGE_SERVER_URL ONTOFORGE_TOKEN ONTOFORGE_NO_GRANT_TOKEN; do
    value=${!key:-}
    [[ -n "$value" ]] && printf '%s=%s\n' "$key" "$value" >> "$ENV_FILE"
  done
}

load_env() {
  if [[ -f "$ENV_FILE" ]]; then
    # shellcheck disable=SC1090
    source "$ENV_FILE"
  fi
}

require_env() {
  local name
  for name in "$@"; do
    [[ -n "${!name:-}" ]] || die "$name is required"
  done
}

wait_http() {
  local url=$1
  local deadline=$((SECONDS + 120))
  until curl -fsS --max-time 3 "$url" >/dev/null 2>&1; do
    (( SECONDS < deadline )) || die "timed out waiting for $url"
    sleep 2
  done
}

discover() {
  local package=$1 test_target=$2 pattern=$3 output count
  output=$(cargo test -p "$package" --test "$test_target" --locked -- --ignored "$pattern" --list 2>&1)
  count=$(printf '%s\n' "$output" | awk '/: test$/ { n++ } END { print n+0 }')
  [[ "$count" -gt 0 ]] || die "$test_target discovered zero ignored tests for $pattern"
  printf '%s\n' "$output"
}

start() {
  : > "$ENV_FILE"
  export GITHUB_ENV="$ENV_FILE"
  export RUNNER_TEMP
  export PGHOST=localhost
  export PGUSER=postgres
  bash "$ROOT/ci/query-services.sh" postgres
  load_env
  require_env ONTOFORGE_PG_URL TRINO_CATALOG_DIR
  for database in demo editstore; do
    if ! psql -v ON_ERROR_STOP=1 -d postgres -Atqc "SELECT 1 FROM pg_database WHERE datname = '$database'" | grep -q '^1$'; then
      psql -v ON_ERROR_STOP=1 -d postgres -c "CREATE DATABASE $database"
    fi
  done
  psql -v ON_ERROR_STOP=1 -v fixtures="$ROOT/examples/ontologies/aviation/bindings/dev/fixtures" -d demo -f "$ROOT/ci/pg/seed.sql"
  psql -v ON_ERROR_STOP=1 -d demo -f "$ROOT/ci/pg/rowpred_corpus.sql"
  psql -v ON_ERROR_STOP=1 -d demo -f "$ROOT/ci/pg/query_corpus.sql"
  bash "$ROOT/ci/query-services.sh" trino
  export ONTOFORGE_TRINO_URL="http://127.0.0.1:8080"
  bash "$ROOT/ci/query-services.sh" trino-l2
  export ONTOFORGE_TRINO_L2_URL="http://127.0.0.1:58081"
  export ONTOFORGE_TRINO_L2_LOOSE_URL="http://127.0.0.1:58082"
  write_env

  cat > "$SERVER_CONFIG" <<'EOF'
[store]
kind = "postgres"
url_env = "ONTOFORGE_PG_URL"
schema_edit = "ontoforge_edit"
schema_sys = "ontoforge_sys"
[store.trino]
catalog = "editstore"
[trino]
url = "http://127.0.0.1:8080"
connect_timeout_ms = 5000
[trino.identities.ontology]
user = "ontoforge"
auth = "none"
[profiles.dev]
engine = "trino"
[auth.dev]
key_file = "dev-token.key"
[query]
signing_key_file = "query-signing.key"
EOF
  openssl rand -out "$STATE_DIR/query-signing.key" 48
  touch "$STATE_DIR/dev-token.key"
  cargo build -p ontoforge --locked
  (cd "$ROOT" && target/debug/ontoforge serve --server-config "$SERVER_CONFIG" --dev --bind 127.0.0.1:18080 >"$SERVER_LOG" 2>&1 & echo $! >"$SERVER_PID_FILE")
  export ONTOFORGE_SERVER_URL="http://127.0.0.1:18080"
  export ONTOFORGE_TOKEN="$(target/debug/ontoforge dev-token --server-config "$SERVER_CONFIG" --sub alice --groups dispatchers)"
  export ONTOFORGE_NO_GRANT_TOKEN="$(target/debug/ontoforge dev-token --server-config "$SERVER_CONFIG" --sub bob)"
  write_env
  wait_http "$ONTOFORGE_SERVER_URL/health"
  log "server ready at $ONTOFORGE_SERVER_URL"
}

verify() {
  load_env
  require_env ONTOFORGE_PG_URL ONTOFORGE_TRINO_URL ONTOFORGE_TRINO_L2_URL ONTOFORGE_TRINO_L2_LOOSE_URL ONTOFORGE_SERVER_URL ONTOFORGE_TOKEN ONTOFORGE_NO_GRANT_TOKEN
  discover ontoforge-server postgres_http_it pg_http_
  cargo test -p ontoforge-server --test postgres_http_it --locked -- --ignored --nocapture --test-threads=1
  cargo test -p ontoforge-storage --test request_snapshot --locked -- --ignored --nocapture --test-threads=1
  discover ontoforge-server trino_http_it trino_http_
  cargo test -p ontoforge-server --test trino_http_it --locked -- --ignored --nocapture --test-threads=1
  discover ontoforge-server health_trino_it real_
  cargo test -p ontoforge-server --test health_trino_it --locked -- --ignored --nocapture --test-threads=1
  cargo test -p ontoforge-engine --test trino_l2_acl_it --locked -- --ignored --nocapture --test-threads=1
  npm --prefix "$ROOT/clients/typescript" ci
  npm --prefix "$ROOT/clients/typescript" run check:generated
  npm --prefix "$ROOT/clients/typescript" run typecheck
  npm --prefix "$ROOT/clients/typescript" run build
  npm --prefix "$ROOT/clients/typescript" run test:contract
}

cleanup() {
  if [[ -f "$SERVER_PID_FILE" ]]; then
    kill "$(cat "$SERVER_PID_FILE")" 2>/dev/null || true
    rm -f "$SERVER_PID_FILE"
  fi
  bash "$ROOT/ci/query-services.sh" cleanup || true
}

case "${1:-}" in
  start) start ;;
  verify) trap cleanup EXIT; start; verify ;;
  cleanup) cleanup ;;
  *) echo "usage: bash ci/server-services.sh {start|verify|cleanup}" >&2; exit 2 ;;
esac
