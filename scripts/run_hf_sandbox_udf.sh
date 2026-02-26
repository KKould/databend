#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

BUILD_PROFILE="${BUILD_PROFILE:-debug}"
META_CONFIG="${META_CONFIG:-$ROOT_DIR/scripts/ci/deploy/config/databend-meta-node-1.toml}"
QUERY_CONFIG="${QUERY_CONFIG:-$ROOT_DIR/scripts/ci/deploy/config/databend-query-node-1.toml}"
SQL_FILE="${SQL_FILE:-$ROOT_DIR/scripts/hf_sandbox_udf_test.sql}"

BENDSQL_HOST="${BENDSQL_HOST:-127.0.0.1}"
BENDSQL_PORT="${BENDSQL_PORT:-8000}"
BENDSQL_USER="${BENDSQL_USER:-root}"
BENDSQL_PASSWORD="${BENDSQL_PASSWORD:-}"

DATA_DIR="${DATA_DIR:-$ROOT_DIR/.databend}"
START_MOCK_SERVER="${START_MOCK_SERVER:-1}"
RUN_CLIENT="${RUN_CLIENT:-1}"
CLEAN_OLD="${CLEAN_OLD:-1}"

UDF_RUNTIME_IMAGE="${UDF_RUNTIME_IMAGE:-databend-udf-runtime:hf-fast}"
AUTO_BUILD_RUNTIME_IMAGE="${AUTO_BUILD_RUNTIME_IMAGE:-1}"

META_BIN="$ROOT_DIR/target/$BUILD_PROFILE/databend-meta"
QUERY_BIN="$ROOT_DIR/target/$BUILD_PROFILE/databend-query"

usage() {
    cat <<'EOF'
Usage: scripts/run_hf_sandbox_udf.sh [options]

Options:
  --sql-file <path>        SQL file to execute (default: scripts/hf_sandbox_udf_test.sql)
  --user <name>            bendsql user (default: root)
  --password <value>       bendsql password (default: empty)
  --build-profile <name>   debug or release (default: debug)
  --no-mock                do not start cloud-control mock server
  --no-client              do not open interactive bendsql after registration
  --no-clean               do not kill old databend/meta/mock processes
  --no-build-image         do not auto-build runtime image if missing
  -h, --help               show help

Environment overrides:
  UDF_RUNTIME_IMAGE, START_MOCK_SERVER, RUN_CLIENT, CLEAN_OLD,
  BENDSQL_HOST, BENDSQL_PORT, BENDSQL_USER, BENDSQL_PASSWORD,
  META_CONFIG, QUERY_CONFIG, SQL_FILE
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --sql-file)
            SQL_FILE="$2"
            shift 2
            ;;
        --user)
            BENDSQL_USER="$2"
            shift 2
            ;;
        --password)
            BENDSQL_PASSWORD="$2"
            shift 2
            ;;
        --build-profile)
            BUILD_PROFILE="$2"
            META_BIN="$ROOT_DIR/target/$BUILD_PROFILE/databend-meta"
            QUERY_BIN="$ROOT_DIR/target/$BUILD_PROFILE/databend-query"
            shift 2
            ;;
        --no-mock)
            START_MOCK_SERVER=0
            shift
            ;;
        --no-client)
            RUN_CLIENT=0
            shift
            ;;
        --no-clean)
            CLEAN_OLD=0
            shift
            ;;
        --no-build-image)
            AUTO_BUILD_RUNTIME_IMAGE=0
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "Unknown option: $1" >&2
            usage
            exit 1
            ;;
    esac
done

need_cmd() {
    if ! command -v "$1" >/dev/null 2>&1; then
        echo "Missing required command: $1" >&2
        exit 1
    fi
}

wait_port() {
    local port="$1"
    local timeout="${2:-60}"
    python3 "$ROOT_DIR/scripts/ci/wait_tcp.py" --timeout "$timeout" --port "$port"
}

register_udf() {
    local sql
    sql="$(cat "$SQL_FILE")"
    local cmd=(bendsql --host "$BENDSQL_HOST" --port "$BENDSQL_PORT" --user "$BENDSQL_USER" --non-interactive --query="$sql")
    if [[ -n "$BENDSQL_PASSWORD" ]]; then
        cmd+=(--password "$BENDSQL_PASSWORD")
    fi
    "${cmd[@]}"
}

open_bendsql_client() {
    local cmd=(bendsql --host "$BENDSQL_HOST" --port "$BENDSQL_PORT" --user "$BENDSQL_USER")
    if [[ -n "$BENDSQL_PASSWORD" ]]; then
        cmd+=(--password "$BENDSQL_PASSWORD")
    fi
    exec "${cmd[@]}"
}

need_cmd python3
need_cmd bendsql

if [[ ! -f "$SQL_FILE" ]]; then
    echo "SQL file not found: $SQL_FILE" >&2
    exit 1
fi
if [[ ! -x "$META_BIN" || ! -x "$QUERY_BIN" ]]; then
    echo "Missing databend binaries, please build first:" >&2
    echo "  make build" >&2
    echo "Expected:" >&2
    echo "  $META_BIN" >&2
    echo "  $QUERY_BIN" >&2
    exit 1
fi

mkdir -p "$DATA_DIR"

if [[ "$CLEAN_OLD" == "1" ]]; then
    echo "[1/6] Cleaning old processes"
    killall -9 databend-query >/dev/null 2>&1 || true
    killall -9 databend-meta >/dev/null 2>&1 || true
    pkill -f "tests/cloud_control_server/simple_server.py" >/dev/null 2>&1 || true
fi

if [[ "$START_MOCK_SERVER" == "1" ]]; then
    need_cmd uv
    need_cmd docker
    if ! docker image inspect "$UDF_RUNTIME_IMAGE" >/dev/null 2>&1; then
        if [[ "$AUTO_BUILD_RUNTIME_IMAGE" != "1" ]]; then
            echo "Runtime image not found: $UDF_RUNTIME_IMAGE" >&2
            echo "Build it first or remove --no-build-image." >&2
            exit 1
        fi
        echo "[2/6] Building runtime image: $UDF_RUNTIME_IMAGE"
        docker build -f "$ROOT_DIR/docker/service/udf-sandbox-runtime.Dockerfile" -t "$UDF_RUNTIME_IMAGE" "$ROOT_DIR"
    else
        echo "[2/6] Runtime image exists: $UDF_RUNTIME_IMAGE"
    fi

    echo "[3/6] Starting cloud-control mock server"
    nohup env UDF_DOCKER_BASE_IMAGE="$UDF_RUNTIME_IMAGE" PYTHONUNBUFFERED=1 \
        uv run --project "$ROOT_DIR/tests/cloud_control_server" \
        python "$ROOT_DIR/tests/cloud_control_server/simple_server.py" \
        >"$DATA_DIR/cloud-control.out" 2>&1 &
    wait_port 50051 60
fi

echo "[4/6] Starting databend-meta"
nohup "$META_BIN" -c "$META_CONFIG" >"$DATA_DIR/meta-1.out" 2>&1 &
wait_port 9191 60

echo "[5/6] Starting databend-query"
nohup env RUST_BACKTRACE=1 "$QUERY_BIN" -c "$QUERY_CONFIG" >"$DATA_DIR/query-1.out" 2>&1 &
wait_port 9091 60
wait_port 8000 60

echo "[6/6] Registering UDF from: $SQL_FILE"
register_udf

echo "Done. Logs:"
echo "  $DATA_DIR/meta-1.out"
echo "  $DATA_DIR/query-1.out"
if [[ "$START_MOCK_SERVER" == "1" ]]; then
    echo "  $DATA_DIR/cloud-control.out"
fi

if [[ "$RUN_CLIENT" == "1" ]]; then
    echo "Opening bendsql interactive client..."
    open_bendsql_client
else
    echo "You can connect with:"
    echo "  bendsql --host $BENDSQL_HOST --port $BENDSQL_PORT --user $BENDSQL_USER"
fi
