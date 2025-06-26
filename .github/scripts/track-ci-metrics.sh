#!/usr/bin/env bash
# Script to track and report CI performance metrics

set -euo pipefail

# Configuration
METRICS_FILE="${GITHUB_WORKSPACE:-.}/ci-metrics.json"
SUMMARY_FILE="${GITHUB_STEP_SUMMARY:-/dev/stdout}"

# Initialize metrics file if it doesn't exist
if [ ! -f "$METRICS_FILE" ]; then
    echo '{"builds": [], "tests": []}' > "$METRICS_FILE"
fi

# Function to record build time
record_build_time() {
    local job_name="$1"
    local duration="$2"
    local target="${3:-}"
    local cache_hit="${4:-false}"
    
    jq --arg job "$job_name" \
       --arg dur "$duration" \
       --arg target "$target" \
       --arg cache "$cache_hit" \
       --arg date "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
       '.builds += [{
           "job": $job,
           "duration_seconds": ($dur | tonumber),
           "target": $target,
           "cache_hit": ($cache | test("true")),
           "timestamp": $date
       }]' "$METRICS_FILE" > "${METRICS_FILE}.tmp" && mv "${METRICS_FILE}.tmp" "$METRICS_FILE"
}

# Function to record test time
record_test_time() {
    local test_name="$1"
    local duration="$2"
    local passed="${3:-true}"
    
    jq --arg test "$test_name" \
       --arg dur "$duration" \
       --arg pass "$passed" \
       --arg date "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
       '.tests += [{
           "test": $test,
           "duration_seconds": ($dur | tonumber),
           "passed": ($pass | test("true")),
           "timestamp": $date
       }]' "$METRICS_FILE" > "${METRICS_FILE}.tmp" && mv "${METRICS_FILE}.tmp" "$METRICS_FILE"
}

# Function to generate performance summary
generate_summary() {
    echo "## CI Performance Report 📊" >> "$SUMMARY_FILE"
    echo "" >> "$SUMMARY_FILE"
    
    # Calculate averages for the last 10 builds
    local avg_build_time=$(jq '[.builds[-10:][].duration_seconds] | add/length | floor' "$METRICS_FILE" 2>/dev/null || echo "N/A")
    local avg_test_time=$(jq '[.tests[-10:][].duration_seconds] | add/length | floor' "$METRICS_FILE" 2>/dev/null || echo "N/A")
    local cache_hit_rate=$(jq '[.builds[-10:][] | select(.cache_hit == true)] | length * 100 / 10' "$METRICS_FILE" 2>/dev/null || echo "0")
    
    echo "### Average Performance (Last 10 Runs)" >> "$SUMMARY_FILE"
    echo "- **Build Time**: ${avg_build_time}s" >> "$SUMMARY_FILE"
    echo "- **Test Time**: ${avg_test_time}s" >> "$SUMMARY_FILE"
    echo "- **Cache Hit Rate**: ${cache_hit_rate}%" >> "$SUMMARY_FILE"
    echo "" >> "$SUMMARY_FILE"
    
    # Recent builds table
    echo "### Recent Builds" >> "$SUMMARY_FILE"
    echo "| Job | Duration | Target | Cache Hit |" >> "$SUMMARY_FILE"
    echo "|-----|----------|--------|-----------|" >> "$SUMMARY_FILE"
    
    jq -r '.builds[-5:][] | "| \(.job) | \(.duration_seconds)s | \(.target // "N/A") | \(.cache_hit) |"' "$METRICS_FILE" >> "$SUMMARY_FILE" 2>/dev/null || true
    
    echo "" >> "$SUMMARY_FILE"
    
    # Test performance
    echo "### Test Performance" >> "$SUMMARY_FILE"
    echo "| Test | Duration | Status |" >> "$SUMMARY_FILE"
    echo "|------|----------|--------|" >> "$SUMMARY_FILE"
    
    jq -r '.tests[-10:][] | "| \(.test) | \(.duration_seconds)s | \(if .passed then "✅" else "❌" end) |"' "$METRICS_FILE" >> "$SUMMARY_FILE" 2>/dev/null || true
}

# Function to check performance thresholds
check_thresholds() {
    local build_threshold="${BUILD_TIME_THRESHOLD:-300}"
    local test_threshold="${TEST_TIME_THRESHOLD:-180}"
    
    local max_build_time=$(jq '[.builds[-5:][].duration_seconds] | max' "$METRICS_FILE" 2>/dev/null || echo "0")
    local max_test_time=$(jq '[.tests[-5:][].duration_seconds] | max' "$METRICS_FILE" 2>/dev/null || echo "0")
    
    if [ "${max_build_time%.*}" -gt "$build_threshold" ]; then
        echo "::warning::Build time exceeded threshold: ${max_build_time}s > ${build_threshold}s"
    fi
    
    if [ "${max_test_time%.*}" -gt "$test_threshold" ]; then
        echo "::warning::Test time exceeded threshold: ${max_test_time}s > ${test_threshold}s"
    fi
}

# Function to cleanup old metrics
cleanup_metrics() {
    local days_to_keep="${METRICS_RETENTION_DAYS:-30}"
    local cutoff_date=$(date -u -d "$days_to_keep days ago" +%Y-%m-%dT%H:%M:%SZ 2>/dev/null || \
                       date -u -v-${days_to_keep}d +%Y-%m-%dT%H:%M:%SZ)
    
    jq --arg cutoff "$cutoff_date" '
        .builds |= map(select(.timestamp > $cutoff)) |
        .tests |= map(select(.timestamp > $cutoff))
    ' "$METRICS_FILE" > "${METRICS_FILE}.tmp" && mv "${METRICS_FILE}.tmp" "$METRICS_FILE"
}

# Main command handling
case "${1:-help}" in
    record-build)
        record_build_time "$2" "$3" "${4:-}" "${5:-false}"
        ;;
    record-test)
        record_test_time "$2" "$3" "${4:-true}"
        ;;
    summary)
        generate_summary
        ;;
    check)
        check_thresholds
        ;;
    cleanup)
        cleanup_metrics
        ;;
    *)
        echo "Usage: $0 {record-build|record-test|summary|check|cleanup}"
        echo ""
        echo "Commands:"
        echo "  record-build <job> <duration> [target] [cache_hit]"
        echo "  record-test <test> <duration> [passed]"
        echo "  summary          Generate performance summary"
        echo "  check           Check against thresholds"
        echo "  cleanup         Remove old metrics"
        exit 1
        ;;
esac