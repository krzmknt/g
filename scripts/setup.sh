#!/usr/bin/env bash
set -euo pipefail

BOLD='\033[1m'
RESET='\033[0m'

echo -e "${BOLD}=== crates.io publish workflow setup ===${RESET}\n"

# Check gh CLI
if ! command -v gh &> /dev/null; then
    echo "Error: gh CLI is not installed. Install it from https://cli.github.com"
    exit 1
fi

# Check gh auth
if ! gh auth status &> /dev/null; then
    echo "Error: Not authenticated with gh. Run 'gh auth login' first."
    exit 1
fi

# Check repo context
REPO=$(gh repo view --json nameWithOwner -q '.nameWithOwner' 2>/dev/null) || true
if [ -z "$REPO" ]; then
    echo "Error: Not in a GitHub repository or remote is not configured."
    exit 1
fi

echo "Repository: $REPO"
echo ""

# Check if secret already exists
if gh secret list | grep -q "CARGO_REGISTRY_TOKEN"; then
    echo "CARGO_REGISTRY_TOKEN is already configured."
    read -rp "Overwrite? (y/N): " answer
    if [[ ! "$answer" =~ ^[Yy]$ ]]; then
        echo "Skipped."
        exit 0
    fi
fi

echo ""
echo "Enter your crates.io API token."
echo "You can create one at: https://crates.io/settings/tokens"
echo ""
read -rsp "Token: " token
echo ""

if [ -z "$token" ]; then
    echo "Error: Token cannot be empty."
    exit 1
fi

echo "$token" | gh secret set CARGO_REGISTRY_TOKEN

echo ""
echo "CARGO_REGISTRY_TOKEN has been set for $REPO."
