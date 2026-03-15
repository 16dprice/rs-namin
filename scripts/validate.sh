#!/usr/bin/env bash
set -euo pipefail

passed=0
failed=0
failures=()

run_step() {
    local name="$1"
    shift
    printf "  %-20s" "$name"
    if output=$("$@" 2>&1); then
        echo "OK"
        passed=$((passed + 1))
    else
        echo "FAIL"
        failed=$((failed + 1))
        failures+=("--- $name ---"$'\n'"$output")
    fi
}

echo "Running validation..."
echo ""

run_step "cargo fmt" cargo fmt --check
run_step "cargo clippy" cargo clippy -- -D warnings
run_step "cargo test" cargo test

echo ""
if [ "$failed" -eq 0 ]; then
    echo "All $passed checks passed."
else
    echo "$failed/$((passed + failed)) checks failed:"
    echo ""
    for f in "${failures[@]}"; do
        echo "$f"
        echo ""
    done
    exit 1
fi
