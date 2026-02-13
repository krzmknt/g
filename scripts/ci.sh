#!/usr/bin/env bash
set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
BOLD='\033[1m'
RESET='\033[0m'

pass() { echo -e "${GREEN}PASS${RESET} $1"; }
fail() { echo -e "${RED}FAIL${RESET} $1"; exit 1; }
step() { echo -e "\n${BOLD}--- $1 ---${RESET}"; }

step "cargo fmt --check"
if cargo fmt --check; then
    pass "formatting"
else
    fail "formatting"
fi

step "cargo clippy -- -D warnings"
if cargo clippy -- -D warnings; then
    pass "clippy"
else
    fail "clippy"
fi

step "cargo test"
if cargo test; then
    pass "tests"
else
    fail "tests"
fi

# Version check: only when a tag argument is provided
if [ "${1:-}" != "" ]; then
    step "Version check (tag: $1)"
    TAG_VERSION="${1#v}"
    CARGO_VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
    if [ "$TAG_VERSION" = "$CARGO_VERSION" ]; then
        pass "tag v$TAG_VERSION matches Cargo.toml $CARGO_VERSION"
    else
        fail "tag v$TAG_VERSION does not match Cargo.toml $CARGO_VERSION"
    fi
fi

echo -e "\n${GREEN}${BOLD}All checks passed.${RESET}"
