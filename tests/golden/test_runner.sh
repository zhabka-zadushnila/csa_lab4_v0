#!/usr/bin/env bash
# Golden test runner for lab4
# Usage: ./tests/golden/test_runner.sh [test_name]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
COMPILER="$ROOT/compiler/target/release/compiler"
PSIM="$ROOT/psim/target/release/psim"
GOLDEN_DIR="$ROOT/tests/golden"
TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

PASS=0
FAIL=0

run_one_test() {
    local testdir="$1"
    local name="$(basename "$testdir")"
    local source="$testdir/source.algo"
    local input="$testdir/input.txt"
    local expected_out="$testdir/expected_output.txt"
    local expected_log="$testdir/expected_log.txt"

    if [ ! -f "$source" ]; then
        return 0
    fi

    printf "Golden test: %-20s " "$name"

    local bin="$TMPDIR/$name.bin"
    local list="$TMPDIR/$name.list"
    local out="$TMPDIR/$name.out"
    local log="$TMPDIR/$name.log"
    local full="$TMPDIR/$name.full"

    local ast_actual="$TMPDIR/$name.ast"

    # Compile
    if ! "$COMPILER" "$source" "$bin" --listing "$list" --ast "$ast_actual" > /dev/null 2>&1; then
        echo "FAIL (compilation)"
        return 1
    fi

    # Check AST
    local expected_ast="$testdir/expected_ast.txt"
    if [ -f "$expected_ast" ]; then
        if ! diff -q "$expected_ast" "$ast_actual" > /dev/null 2>&1; then
            echo "FAIL (AST)"
            echo "--- expected AST ---"
            cat "$expected_ast"
            echo "--- actual AST ---"
            cat "$ast_actual"
            return 1
        fi
    fi

    # Simulate
    local psim_args="$bin"
    if [ -f "$input" ]; then
        psim_args="$bin $input"
    fi

    if ! "$PSIM" $psim_args > "$full" 2>&1; then
        echo "FAIL (simulation)"
        return 1
    fi

    # Separate log (TICK lines) from output (non-TICK lines)
    grep "^TICK" "$full" > "$log" || true
    grep -v "^TICK" "$full" | awk 'length' > "$out" || true

    # Check output
    if [ -f "$expected_out" ]; then
        if ! diff -q "$expected_out" "$out" > /dev/null 2>&1; then
            echo "FAIL (output)"
            echo "--- expected ---"
            cat "$expected_out"
            echo "--- actual ---"
            cat "$out"
            return 1
        fi
    fi

    # Check log (trimmed: head + tail + lines with HLT/JMP/MAP)
    if [ -f "$expected_log" ]; then
        {
            head -5 "$log"
            grep -E "(MAP|JMP.*fetch|HLT)" "$log" | head -20
            tail -3 "$log"
        } > "$TMPDIR/${name}_log_trimmed.txt" || true

        if ! diff -q "$expected_log" "$TMPDIR/${name}_log_trimmed.txt" > /dev/null 2>&1; then
            echo "FAIL (log)"
            echo "--- expected log ---"
            cat "$expected_log"
            echo "--- actual trimmed log ---"
            cat "$TMPDIR/${name}_log_trimmed.txt"
            echo "--- full log ($(wc -l < "$log") lines) ---"
            return 1
        fi
    fi

    echo "PASS"
    return 0
}

if [ $# -gt 0 ]; then
    testdir="$GOLDEN_DIR/$1"
    if [ -d "$testdir" ]; then
        run_one_test "$testdir" && PASS=$((PASS + 1)) || FAIL=$((FAIL + 1))
    else
        echo "Test '$1' not found"
        exit 1
    fi
else
    for testdir in "$GOLDEN_DIR"/*/; do
        run_one_test "$testdir" && PASS=$((PASS + 1)) || FAIL=$((FAIL + 1))
    done
fi

echo ""
echo "Results: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ]
