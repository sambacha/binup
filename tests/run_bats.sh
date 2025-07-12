#!/usr/bin/env bash
set -euo pipefail

# Simple BATS test runner for gup shell integration tests

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$PROJECT_ROOT"

# Color output helpers
if [[ -t 1 ]]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[0;33m'
    NC='\033[0m' # No Color
else
    RED=''
    GREEN=''
    YELLOW=''
    NC=''
fi

echo_error() {
    echo -e "${RED}Error: $1${NC}" >&2
}

echo_success() {
    echo -e "${GREEN}$1${NC}"
}

echo_info() {
    echo -e "${YELLOW}$1${NC}"
}

# Check for BATS
if ! command -v bats &> /dev/null; then
    echo_error "BATS not found. Please install bats-core."
    echo
    echo "Installation instructions:"
    echo "  macOS:    brew install bats-core"
    echo "  Ubuntu:   sudo apt-get install bats"
    echo "  Fedora:   sudo dnf install bats"
    echo "  From source: https://github.com/bats-core/bats-core"
    exit 1
fi

# Build debug binary if needed or if --rebuild is passed
if [[ ! -f "target/debug/gup" ]] || [[ "${1:-}" == "--rebuild" ]]; then
    echo_info "Building debug binary..."
    if cargo build --quiet; then
        echo_success "Build successful"
    else
        echo_error "Build failed"
        exit 1
    fi
    
    # Shift the --rebuild argument if it was provided
    [[ "${1:-}" == "--rebuild" ]] && shift
fi

# Check if specific test file was provided
if [[ -n "${1:-}" ]]; then
    TEST_FILES="$@"
else
    TEST_FILES="tests/bats/shell_integration.bats"
fi

# Run tests
echo_info "Running BATS tests..."
echo

if bats $TEST_FILES; then
    echo
    echo_success "All tests passed!"
else
    echo
    echo_error "Some tests failed"
    exit 1
fi