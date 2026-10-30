#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$ROOT"
export CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-2}
export RUNNER_TEMP=${RUNNER_TEMP:-"$ROOT/target/phase7-services/runtime"}
EVIDENCE_DIR="$ROOT/target/phase7-services/evidence"
ENV_FILE="$RUNNER_TEMP/server-services.env"
SERVER_CONFIG="$RUNNER_TEMP/ontoforge-server.toml"
SERVER_PID_FILE="$RUNNER_TEMP/server.pid"
STARTED_FILE="$RUNNER_TEMP/services.started"
BINARY="$ROOT/target/debug/ontoforge"
CURRENT_RUN_STARTED=0
if [[ "${OS:-}" == Windows_NT ]]; then BINARY+=.exe; fi
mkdir -p "$RUNNER_TEMP" "$EVIDENCE_DIR"
chmod 700 "$RUNNER_TEMP"
log() { printf '[server-services] %s\n' "$*"; }
die() { printf '[server-services] ERROR: %s\n' "$*" >&2; exit 1; }

load_env() {
  if [[ -f "$ENV_FILE" ]]; then
    set -a
    # shellcheck disable=SC1090
    source "$ENV_FILE"
    set +a
  fi
}

write_env() {
  local key value
  for key in PGHOST PGUSER TRINO_CATALOG_DIR PGPASSWORD ONTOFORGE_PG_URL ONTOFORGE_TRINO_URL ONTOFORGE_TRINO_L2_URL ONTOFORGE_TRINO_L2_LOOSE_URL ONTOFORGE_SERVER_URL ONTOFORGE_SERVER_CONFIG ONTOFORGE_TOKEN ONTOFORGE_NO_GRANT_TOKEN ONTOFORGE_L2_TOKEN; do
    value=${!key:-}
    [[ -z "$value" ]] || printf '%s=%q\n' "$key" "$value" >> "$ENV_FILE"
  done
}

require_env() {
  local name
  for name in "$@"; do
    [[ -n "${!name:-}" ]] || die "$name is required"
  done
}

wait_http() {
  local url=$1 deadline=$((SECONDS + 180))
  until curl -fsS --max-time 3 "$url" >/dev/null 2>&1; do
    (( SECONDS < deadline )) || die "timed out waiting for $url"
    if [[ -f "$SERVER_PID_FILE" ]] && ! kill -0 "$(cat "$SERVER_PID_FILE")" 2>/dev/null; then
      die "server exited before becoming ready; see server.log"
    fi
    sleep 2
  done
}

run_ignored() {
  local package=$1 target=$2; shift 2
  local output name
  output=$(cargo test -p "$package" --test "$target" --locked -- --ignored --list | tr -d '\r')
  printf '%s\n' "$output" > "$EVIDENCE_DIR/$target.discovery.log"
  for name in "$@"; do
    grep -Fxq "$name: test" <<< "$output" || die "$target did not discover required test $name"
  done
  cargo test -p "$package" --test "$target" --locked -- --ignored --nocapture --test-threads=1 2>&1 | tee "$EVIDENCE_DIR/$target.log"
}

start() {
  local command container
  for command in docker cargo psql jq openssl node npm curl; do
    command -v "$command" >/dev/null || die "$command is required"
  done
  docker info >/dev/null
  for container in postgres trino ontoforge-trino-l2 ontoforge-trino-l2-loose; do
    if docker inspect "$container" >/dev/null 2>&1; then
      die "container $container already exists; refusing to replace an unrelated service"
    fi
  done
  CURRENT_RUN_STARTED=1
  : > "$ENV_FILE"
  chmod 600 "$ENV_FILE"
  : > "$STARTED_FILE"
  export PGHOST=localhost PGUSER=postgres
  GITHUB_ENV="$ENV_FILE" bash "$ROOT/ci/query-services.sh" postgres
  load_env
  require_env ONTOFORGE_PG_URL TRINO_CATALOG_DIR PGPASSWORD
  psql -v ON_ERROR_STOP=1 -d postgres -c 'CREATE DATABASE demo' -c 'CREATE DATABASE editstore'
  psql -v ON_ERROR_STOP=1 -v fixtures="$ROOT/examples/ontologies/aviation/bindings/dev/fixtures" -d demo -f "$ROOT/ci/pg/seed.sql"
  psql -v ON_ERROR_STOP=1 -d demo -f "$ROOT/ci/pg/rowpred_corpus.sql"
  psql -v ON_ERROR_STOP=1 -d demo -f "$ROOT/ci/pg/query_corpus.sql"
  bash "$ROOT/ci/query-services.sh" trino
  export ONTOFORGE_TRINO_URL="http://127.0.0.1:8080"
  local deadline=$((SECONDS + 180))
  until curl -fsS --max-time 3 "$ONTOFORGE_TRINO_URL/v1/info" | jq -e '.starting == false and .nodeVersion.version == "483"' >/dev/null; do
    (( SECONDS < deadline )) || die "Trino 483 did not finish starting"
    sleep 2
  done
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
[auth.principal_attrs.homeAirports]
claim = "airports"
type = "string_list"
[auth.dev]
key_file = "dev-token.key"
[query]
signing_key_file = "query-signing.key"
EOF
  cargo build -p ontoforge --locked
  "$BINARY" reload "$ROOT/examples/ontologies/aviation" --server-config "$SERVER_CONFIG" --profile dev --format json > "$EVIDENCE_DIR/reload.json"
  local plan_hash
  plan_hash=$("$BINARY" scope plan "$ROOT/examples/scopes/aviation" --ontology "$ROOT/examples/ontologies/aviation" --server-config "$SERVER_CONFIG" --format json | jq -er '.result.plan.hash')
  [[ "$plan_hash" =~ ^[0-9a-f]{16}$ ]] || die "scope plan did not return a valid hash"
  "$BINARY" scope apply "$ROOT/examples/scopes/aviation" --ontology "$ROOT/examples/ontologies/aviation" --server-config "$SERVER_CONFIG" --plan-hash "$plan_hash" --format json > "$EVIDENCE_DIR/scope-apply.json"
  openssl rand -out "$RUNNER_TEMP/query-signing.key" 48
  ONTOFORGE_TOKEN=$("$BINARY" dev-token --server-config "$SERVER_CONFIG" --sub alice --groups dispatchers)
  ONTOFORGE_NO_GRANT_TOKEN=$("$BINARY" dev-token --server-config "$SERVER_CONFIG" --sub bob)
  ONTOFORGE_L2_TOKEN=$("$BINARY" dev-token --server-config "$SERVER_CONFIG" --sub mia)
  export ONTOFORGE_TOKEN ONTOFORGE_NO_GRANT_TOKEN ONTOFORGE_L2_TOKEN
  export ONTOFORGE_SERVER_CONFIG="$SERVER_CONFIG"
  export ONTOFORGE_SERVER_URL="http://127.0.0.1:18080"
  write_env
  "$BINARY" serve --server-config "$SERVER_CONFIG" --dev --bind 127.0.0.1:18080 > "$EVIDENCE_DIR/server.log" 2>&1 &
  printf '%s\n' "$!" > "$SERVER_PID_FILE"
  wait_http "$ONTOFORGE_SERVER_URL/health"
  log "PostgreSQL/Trino server ready"
}

verify() {
  load_env
  require_env ONTOFORGE_PG_URL ONTOFORGE_TRINO_URL ONTOFORGE_TRINO_L2_URL ONTOFORGE_TRINO_L2_LOOSE_URL ONTOFORGE_SERVER_URL ONTOFORGE_TOKEN ONTOFORGE_NO_GRANT_TOKEN ONTOFORGE_L2_TOKEN
  git rev-parse HEAD > "$EVIDENCE_DIR/commit.txt"
  run_ignored ontoforge-server postgres_http_it pg_http_mixed_snapshot_reload_revoke_and_history pg_http_action_validate_stale_replay_atomicity
  run_ignored ontoforge-storage request_snapshot snapshot_pg_never_mixes_definition_roles_bindings
  run_ignored ontoforge-server trino_http_it trino_http_long_read_releases_snapshot_lock trino_http_and_mcp_six_l2_tools_all_formats
  run_ignored ontoforge-server health_trino_it real_health_l2_failure_expiry_recovery_keeps_core real_shutdown_action_dependency_audit_and_cleanup
  run_ignored ontoforge-engine trino_l2_acl_it trino_l2_acl_rejects_existing_outside_and_edit_tables trino_l2_acl_loosened_rule_fails_closed
  local client_tests name
  client_tests=$(cargo test -p ontoforge-client --test contract --locked -- --list | tr -d '\r')
  for name in rust_client_metadata_pk_query_aggregate_health rust_client_instances_and_concurrent_cancellation_are_isolated rust_client_action_validate_history_result_and_replay rust_client_auth_scope_revoke_stale_and_transport_errors rust_client_wire_numbers_and_all_endpoint_groups; do
    grep -Fxq "$name: test" <<< "$client_tests" || die "Rust client did not discover required test $name"
  done
  cargo test -p ontoforge-client --test contract --locked -- --nocapture 2>&1 | tee "$EVIDENCE_DIR/rust-client.log"
  npm --prefix "$ROOT/tools/openapi-gen" ci
  npm --prefix "$ROOT/clients/typescript" ci
  npm --prefix "$ROOT/clients/typescript" run check:generated
  npm --prefix "$ROOT/clients/typescript" run typecheck
  npm --prefix "$ROOT/clients/typescript" run build
  npm --prefix "$ROOT/clients/typescript" run test:contract 2>&1 | tee "$EVIDENCE_DIR/typescript-contract.log"
  for name in ts_client_generated_metadata_pk_query_aggregate_health ts_client_typecheck_generation_and_instance_abort ts_lossless_integer_decimal_boundary_roundtrips ts_client_action_validate_history_result_and_replay ts_client_auth_scope_stale_errors_limits_and_runtime_capability; do
    grep -Fq "$name" "$EVIDENCE_DIR/typescript-contract.log" || die "TypeScript contract did not execute $name"
  done
  grep -Eq 'skipped[[:space:]]+0([[:space:]]|$)' "$EVIDENCE_DIR/typescript-contract.log" || die "TypeScript contract skipped tests or did not produce a test summary"
}

check_artifacts() {
  load_env
  require_env PGPASSWORD
  bash "$ROOT/ci/query-services.sh" check-artifacts "$EVIDENCE_DIR"
  local name status
  for name in ONTOFORGE_TOKEN ONTOFORGE_NO_GRANT_TOKEN ONTOFORGE_L2_TOKEN; do
    [[ -n "${!name:-}" ]] || continue
    status=0
    grep -rFq -- "${!name}" "$EVIDENCE_DIR" || status=$?
    [[ "$status" -eq 1 ]] || die "artifact scan failed for $name"
  done
}

cleanup() {
  local failed=0 pid container
  load_env
  if [[ -f "$SERVER_PID_FILE" ]]; then
    pid=$(cat "$SERVER_PID_FILE")
    if kill -0 "$pid" 2>/dev/null; then
      kill -INT "$pid" || failed=1
      local deadline=$((SECONDS + 30))
      while kill -0 "$pid" 2>/dev/null; do
        if (( SECONDS >= deadline )); then
          kill -KILL "$pid" 2>/dev/null || true
          failed=1
          break
        fi
        sleep 1
      done
      local exit_status=0
      wait "$pid" 2>/dev/null || exit_status=$?
      # A separate cleanup invocation cannot wait on the original shell's child.
      [[ "$exit_status" -eq 0 || "$exit_status" -eq 127 ]] || failed=1
    fi
    rm -f "$SERVER_PID_FILE"
  fi
  if [[ -f "$STARTED_FILE" ]]; then
    if ! docker info >/dev/null 2>&1; then
      printf '[server-services] cannot verify container cleanup: Docker is unavailable\n' >&2
      return 1
    fi
    bash "$ROOT/ci/query-services.sh" cleanup || failed=1
    for container in postgres trino ontoforge-trino-l2 ontoforge-trino-l2-loose; do
      if docker inspect "$container" >/dev/null 2>&1; then
        printf '[server-services] cleanup left container %s\n' "$container" >&2
        failed=1
      fi
    done
    rm -f "$STARTED_FILE"
  fi
  return "$failed"
}

finish() {
  local status=$?
  trap - EXIT
  if [[ "$CURRENT_RUN_STARTED" -eq 1 ]]; then
    if [[ -s "$ENV_FILE" ]] && ! (check_artifacts); then status=1; fi
    if ! cleanup; then status=1; fi
  fi
  exit "$status"
}

case "${1:-}" in
  start) start ;;
  verify) trap finish EXIT; trap 'exit 130' INT; trap 'exit 143' TERM; start; verify ;;
  verify-existing) verify; check_artifacts ;;
  check-artifacts) check_artifacts ;;
  cleanup) cleanup ;;
  *) echo "usage: bash ci/server-services.sh {start|verify|verify-existing|check-artifacts|cleanup}" >&2; exit 2 ;;
esac
