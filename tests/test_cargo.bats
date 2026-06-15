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

    # Wait for PostgreSQL to be ready
    for i in $(seq 1 30); do
        if podman exec "${CONTAINER_NAME}" pg_isready -U "${POSTGRES_USER}" 2>/dev/null; then
            break
        fi
        sleep 1
    done

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
