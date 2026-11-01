#!/usr/bin/env bats

CONTAINER_NAME="scan-primary-mirror-test-postgres"
POSTGRES_PASSWORD="test_password"
POSTGRES_USER="test_user"
DB_NAME="test_db"
DB_PORT=5432
HTTP_PORT=17397

setup_file() {
    # Start PostgreSQL container
    podman run --detach \
        --name "${CONTAINER_NAME}" \
        --env POSTGRES_PASSWORD="${POSTGRES_PASSWORD}" \
        --env POSTGRES_USER="${POSTGRES_USER}" \
        --env POSTGRES_DB="${DB_NAME}" \
        --publish "${DB_PORT}:5432" \
        registry.access.redhat.com/hi/postgresql:latest

    # Wait for PostgreSQL to be ready.
    #
    # Ask over TCP and not over the unix socket. While initializing the
    # database the entrypoint starts a temporary server with
    # listen_addresses='' on the same socket and stops it again before
    # starting the real one, so a check on the socket can succeed on a
    # server which is about to go away. The temporary server cannot answer
    # on TCP, which makes this wait for the real one.
    for _ in $(seq 1 60); do
        if podman exec "${CONTAINER_NAME}" \
            pg_isready -h localhost -U "${POSTGRES_USER}" 2>/dev/null; then
            break
        fi
        sleep 1
    done

    if ! podman exec "${CONTAINER_NAME}" \
        pg_isready -h localhost -U "${POSTGRES_USER}"; then
        echo "PostgreSQL did not come up" >&2
        podman logs "${CONTAINER_NAME}" >&2
        return 1
    fi

    # Load schema
    podman exec -i "${CONTAINER_NAME}" psql -U "${POSTGRES_USER}" -d "${DB_NAME}" \
        < test/database-setup.sql

    # Start HTTP server for test file serving
    python3 -m http.server "${HTTP_PORT}" > /dev/null 2>&1 &
    echo "$!" > "${BATS_FILE_TMPDIR}/http_server.pid"
}

teardown_file() {
    podman rm -f "${CONTAINER_NAME}" || true

    # Stop HTTP server
    if [ -f "${BATS_FILE_TMPDIR}/http_server.pid" ]; then
        kill "$(cat "${BATS_FILE_TMPDIR}/http_server.pid")" 2>/dev/null || true
    fi
}

@test "cargo fmt check" {
    cargo fmt --check
}

@test "cargo build" {
    cargo build --verbose
}

@test "cargo clippy" {
    cargo clippy --all-targets --verbose -- -D warnings
}

@test "cargo test" {
    TEST_DATABASE_URL="postgresql://${POSTGRES_USER}:${POSTGRES_PASSWORD}@localhost:${DB_PORT}/${DB_NAME}" \
        cargo test --verbose -- --test-threads=1
}
