#!/usr/bin/env bash

# Platform detection
export TEST_OS="$(uname -s | tr '[:upper:]' '[:lower:]')"

# Get the next test number from counter file
get_next_test_number() {
    local counter_file="$BATS_TEST_DIRNAME/test-runs/counter"
    local current=100000
    
    # Ensure test-runs directory exists
    mkdir -p "$BATS_TEST_DIRNAME/test-runs"
    
    # Read current counter if file exists
    if [[ -f "$counter_file" ]]; then
        current=$(cat "$counter_file" 2>/dev/null || echo 100000)
    fi
    
    # Increment and save
    local next=$((current + 1))
    echo "$next" > "$counter_file"
    
    echo "$current"
}

# Create isolated test environment by copying test harness
setup_test_env() {
    # Save original environment
    export ORIGINAL_HOME="$HOME"
    export ORIGINAL_PATH="$PATH"
    export ORIGINAL_GUP_DEPOT_PATH="${GUP_DEPOT_PATH:-}"
    
    # Get test number and create test directory
    export TEST_NUMBER=$(get_next_test_number)
    export TEST_RUN_DIR="$BATS_TEST_DIRNAME/test-runs/$TEST_NUMBER"
    local template_dir="$BATS_TEST_DIRNAME/test-harness"
    
    # Copy template to numbered test directory
    if [[ -d "$template_dir" ]]; then
        cp -r "$template_dir" "$TEST_RUN_DIR" || {
            echo "# Failed to copy test harness from $template_dir to $TEST_RUN_DIR" >&3
            return 1
        }
    else
        echo "# Test harness template not found at $template_dir" >&3
        return 1
    fi
    
    # Set up test environment
    export TEST_HOME="$TEST_RUN_DIR/home"
    export GUP_DEPOT_PATH="$TEST_RUN_DIR"
    export HOME="$TEST_HOME"
    # Note: gup will create /gup internally when GUP_DEPOT_PATH is set, so bin will be at $TEST_RUN_DIR/gup/bin
    export PATH="$TEST_RUN_DIR/gup/bin:$PATH"
    
    # Ensure cleanup on exit
    trap 'teardown_test_env' EXIT
    
    # Build test binary with error checking
    local cargo_dir="$BATS_TEST_DIRNAME/../.."
    if [[ ! -f "$cargo_dir/target/debug/gup" ]]; then
        echo "# Building gup binary..." >&3
        (cd "$cargo_dir" && cargo build --quiet) || {
            echo "# Failed to build gup binary" >&2
            return 1
        }
    fi
    export GUP_BIN="$cargo_dir/target/debug/gup"
    
    # Log test environment for debugging
    echo "# Test $TEST_NUMBER: HOME=$HOME, GUP_DEPOT_PATH=$GUP_DEPOT_PATH" >&3
}

teardown_test_env() {
    # Restore original environment
    export HOME="$ORIGINAL_HOME"
    export PATH="$ORIGINAL_PATH"
    if [[ -n "$ORIGINAL_GUP_DEPOT_PATH" ]]; then
        export GUP_DEPOT_PATH="$ORIGINAL_GUP_DEPOT_PATH"
    else
        unset GUP_DEPOT_PATH
    fi
    
    # Remove trap
    trap - EXIT
    
    # Optional: Clean up test directory (comment out to preserve for debugging)
    # [[ -d "$TEST_RUN_DIR" ]] && rm -rf "$TEST_RUN_DIR"
    
    # Note: We keep test directories by default for debugging
    # They can be cleaned manually or with: rm -rf tests/bats/test-runs/[0-9]*
}

# Enhanced assertion helpers with better error messages
assert_success() {
    if [[ "$status" -ne 0 ]]; then
        echo "# Test $TEST_NUMBER: Command failed with status $status" >&3
        echo "# Output:" >&3
        echo "$output" | sed 's/^/# /' >&3
        return 1
    fi
}

assert_failure() {
    if [[ "$status" -eq 0 ]]; then
        echo "# Test $TEST_NUMBER: Expected command to fail but it succeeded" >&3
        echo "# Output:" >&3
        echo "$output" | sed 's/^/# /' >&3
        return 1
    fi
}

assert_file_exists() {
    local file="$1"
    if [[ ! -f "$file" ]]; then
        echo "# Test $TEST_NUMBER: File not found: $file" >&3
        return 1
    fi
}

assert_file_contains() {
    local file="$1"
    local content="$2"
    local description="${3:-File should contain expected content}"
    
    if [[ ! -f "$file" ]]; then
        echo "# Test $TEST_NUMBER: $description - File not found: $file" >&3
        return 1
    fi
    
    if ! grep -qF "$content" "$file"; then
        echo "# Test $TEST_NUMBER: $description - File $file does not contain: $content" >&3
        echo "# Actual content:" >&3
        cat "$file" | sed 's/^/# /' >&3
        return 1
    fi
}

assert_file_not_contains() {
    local file="$1"
    local content="$2"
    local description="${3:-File should not contain specified content}"
    
    if [[ ! -f "$file" ]]; then
        # File doesn't exist, so it doesn't contain the content - success
        return 0
    fi
    
    if grep -qF "$content" "$file"; then
        echo "# Test $TEST_NUMBER: $description - File $file should not contain: $content" >&3
        echo "# But it does. Full content:" >&3
        cat "$file" | sed 's/^/# /' >&3
        return 1
    fi
}

assert_output() {
    local expected
    case "$1" in
        --partial)
            expected="$2"
            if [[ "$output" != *"$expected"* ]]; then
                echo "# Test $TEST_NUMBER: Output does not contain: $expected" >&3
                echo "# Actual output:" >&3
                echo "$output" | sed 's/^/# /' >&3
                return 1
            fi
            ;;
        *)
            expected="$1"
            if [[ "$output" != "$expected" ]]; then
                echo "# Test $TEST_NUMBER: Output does not match expected" >&3
                echo "# Expected:" >&3
                echo "$expected" | sed 's/^/# /' >&3
                echo "# Actual:" >&3
                echo "$output" | sed 's/^/# /' >&3
                return 1
            fi
            ;;
    esac
}

assert_shell_can_execute() {
    local shell="$1"
    local file="$2"
    
    # Skip if shell is not available
    if ! command -v "$shell" &>/dev/null; then
        echo "# Test $TEST_NUMBER: Skipping $shell validation - not installed" >&3
        return 0
    fi
    
    # Test that the shell can source the file without errors
    local shell_output
    if shell_output=$("$shell" -c "source $file && echo 'OK'" 2>&1); then
        if [[ "$shell_output" == *"OK"* ]]; then
            return 0
        fi
    fi
    
    echo "# Test $TEST_NUMBER: Shell $shell cannot execute $file" >&3
    echo "# Shell output:" >&3
    echo "$shell_output" | sed 's/^/# /' >&3
    return 1
}

# Helper to run gup with proper binary
run_gup() {
    run "$GUP_BIN" "$@"
}

# Helper to create shell files with content
create_shell_file() {
    local shell_file="$1"
    local content="$2"
    mkdir -p "$(dirname "$shell_file")"
    echo "$content" > "$shell_file"
}

# Helper to get shell-specific PATH modification content
get_expected_path_content() {
    local shell_file="$1"
    local bin_path="$2"
    
    if [[ "$(basename "$shell_file")" == ".zshrc" ]]; then
        echo "path=('$bin_path' \$path)"
    else
        echo "export PATH=$bin_path\${PATH:+:\${PATH}}"
    fi
}