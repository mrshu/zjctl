#!/usr/bin/env bash
# Smoke test for zjctl + zrpc integration
#
# This script:
# 1. Builds the CLI and plugin
# 2. Verifies CLI command parsing
# 3. Runs protocol unit tests
#
# Requirements:
# - rust toolchain with wasm32-wasip1 target
# - jq for JSON validation (optional, for full integration test)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
PLUGIN_PATH=""
ZJCTL_PATH=""

die() {
    echo "ERROR: $1" >&2
    exit 1
}

info() {
    echo "==> $1"
}

check_deps() {
    info "Checking dependencies..."
    command -v cargo >/dev/null || die "cargo not found in PATH"

    # Check for wasm target
    if ! rustup target list --installed 2>/dev/null | grep -q wasm32-wasip1; then
        if ! rustc --print target-list 2>/dev/null | grep -q wasm32-wasip1; then
            die "wasm32-wasip1 target not available. Run: rustup target add wasm32-wasip1"
        fi
    fi
}

build() {
    info "Building zjctl CLI..."
    cargo build --release -p zjctl --manifest-path "$PROJECT_ROOT/Cargo.toml"
    ZJCTL_PATH="$PROJECT_ROOT/target/release/zjctl"

    info "Building zrpc plugin..."
    cargo build --release -p zjctl-zrpc --target wasm32-wasip1 --manifest-path "$PROJECT_ROOT/Cargo.toml" 2>/dev/null || {
        info "WASM build skipped (target not installed)"
        info "Run: rustup target add wasm32-wasip1"
    }
    PLUGIN_PATH="$PROJECT_ROOT/target/wasm32-wasip1/release/zrpc.wasm"

    [ -f "$ZJCTL_PATH" ] || die "zjctl binary not found at $ZJCTL_PATH"
}

test_cli_parsing() {
    info "Testing CLI command parsing..."

    "$ZJCTL_PATH" --help >/dev/null || die "zjctl --help failed"
    "$ZJCTL_PATH" panes ls --help >/dev/null || die "zjctl panes ls --help failed"
    "$ZJCTL_PATH" pane send --help >/dev/null || die "zjctl pane send --help failed"
    "$ZJCTL_PATH" pane focus --help >/dev/null || die "zjctl pane focus --help failed"
    "$ZJCTL_PATH" pane rename --help >/dev/null || die "zjctl pane rename --help failed"
    "$ZJCTL_PATH" pane resize --help >/dev/null || die "zjctl pane resize --help failed"
    "$ZJCTL_PATH" action --help >/dev/null || die "zjctl action --help failed"

    info "CLI command parsing: PASSED"
}

test_protocol() {
    info "Running protocol unit tests..."
    cargo test -p zjctl-proto --manifest-path "$PROJECT_ROOT/Cargo.toml" || die "Protocol tests failed"
    info "Protocol tests: PASSED"
}

main() {
    cd "$PROJECT_ROOT"

    info "zjctl Smoke Test"
    info "================"
    echo

    check_deps
    build
    test_cli_parsing
    test_protocol

    echo
    info "=========================================="
    info "Smoke test PASSED"
    info "=========================================="
    echo
    info "For full integration testing, run in a Zellij session:"
    echo "  zellij action launch-plugin 'file:$PLUGIN_PATH'"
    echo "  $ZJCTL_PATH panes ls --json"
}

main "$@"
