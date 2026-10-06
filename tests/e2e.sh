#!/usr/bin/env bash
# End-to-end tests for the compiler.
#
# For every examples/*.iris: compile with the release binary, then run the
# resulting wasm with the wasmtime CLI and check results against the
# `# EXPECTED:` annotations in the source file itself:
#
#   # EXPECTED: factorial(5) == 120
#   # EXPECTED: simple() traps
#
# Arguments and results are f64. `traps` means the call must trap (e.g. a
# non-void function that falls off the end hits unreachable).
#
# On top of the annotated cases, every example is compiled twice and the two
# WAT outputs must be identical (the compiler must be deterministic), and
# every exported function must have at least one EXPECTED line.
#
# Run directly (tests/e2e.sh) or via `cargo test` (tests/e2e.rs shells out
# to this when wasmtime is on PATH). Exits non-zero on any failure.

set -u

root="$(cd "$(dirname "$0")/.." && pwd)"
iris="$root/target/release/iris"

if ! command -v wasmtime >/dev/null 2>&1; then
    echo "error: wasmtime not found on PATH (brew install wasmtime)" >&2
    exit 2
fi

echo "building release binary..."
cargo build --release --quiet --manifest-path "$root/Cargo.toml" || exit 1

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

pass=0
fail=0
rows=()

row() { # example case PASS|FAIL detail
    rows+=("$(printf '%-16s  %-40s  %-4s  %s' "$1" "$2" "$3" "$4")")
    if [[ "$3" == PASS ]]; then pass=$((pass + 1)); else fail=$((fail + 1)); fi
}

# name(arg, ...) == value   |   name(arg, ...) traps
ann_re='^([a-zA-Z_][a-zA-Z0-9_]*)\(([^)]*)\)[[:space:]]*(==[[:space:]]*(-?[0-9]+(\.[0-9]+)?([eE][-+]?[0-9]+)?)|traps)[[:space:]]*$'

for src in "$root"/examples/*.iris; do
    name="$(basename "$src" .iris)"
    wat="$tmp/$name.wat"
    wat2="$tmp/$name.2.wat"

    if ! "$iris" "$src" -o "$wat" 2>"$tmp/err"; then
        row "$name" compile FAIL "compiler failed: $(head -1 "$tmp/err")"
        continue
    fi

    if "$iris" "$src" -o "$wat2" 2>/dev/null && cmp -s "$wat" "$wat2"; then
        row "$name" determinism PASS "identical WAT on recompile"
    else
        row "$name" determinism FAIL "two compiles produced different WAT"
    fi

    exports="$(grep -oE '\(export "[^"]+"' "$wat" | sed -E 's/.*"([^"]+)"/\1/')"

    tested=" "
    found_any=0
    while IFS= read -r ann; do
        found_any=1
        if [[ ! "$ann" =~ $ann_re ]]; then
            row "$name" "annotation" FAIL "cannot parse: $ann"
            continue
        fi
        fn="${BASH_REMATCH[1]}"
        argstr="${BASH_REMATCH[2]}"
        expect="${BASH_REMATCH[3]}"  # "== value" or "traps"
        want="${BASH_REMATCH[4]}"
        tested="$tested$fn "

        args="$(echo "$argstr" | tr ',' ' ')"
        label="$fn($(echo $argstr))"

        if ! grep -qxF "$fn" <<<"$exports"; then
            row "$name" "$label" FAIL "not exported from module"
            continue
        fi

        # shellcheck disable=SC2086  # args is intentionally word-split
        out="$(wasmtime run -W timeout=5s --invoke "$fn" "$wat" $args 2>"$tmp/err")"
        rc=$?

        if [[ "$expect" == traps ]]; then
            if [[ $rc -ne 0 ]]; then
                row "$name" "$label" PASS "trapped as expected"
            else
                row "$name" "$label" FAIL "expected trap, returned ${out:-nothing}"
            fi
        elif [[ $rc -ne 0 ]]; then
            row "$name" "$label" FAIL "trapped: $(grep -m1 -iE 'trap|deadline|interrupt' "$tmp/err" | sed 's/^ *//')"
        elif [[ -z "$out" ]]; then
            row "$name" "$label" FAIL "no output from wasmtime"
        elif awk -v got="$out" -v want="$want" 'BEGIN { exit (got + 0 == want + 0) ? 0 : 1 }'; then
            row "$name" "$label" PASS "= $out"
        else
            row "$name" "$label" FAIL "got $out, expected $want"
        fi
    done < <(sed -nE 's/^[[:space:]]*#[[:space:]]*EXPECTED:[[:space:]]*//p' "$src")

    if [[ $found_any -eq 0 ]]; then
        row "$name" annotations FAIL "no # EXPECTED: lines in source"
        continue
    fi

    while IFS= read -r exp; do
        [[ -z "$exp" ]] && continue
        case "$tested" in
            *" $exp "*) ;;
            *) row "$name" "$exp" FAIL "exported but has no # EXPECTED: line" ;;
        esac
    done <<<"$exports"
done

echo
for r in "${rows[@]}"; do
    echo "$r"
done
echo
echo "total: $pass/$((pass + fail)) passed, $fail failed"

[[ $fail -eq 0 ]]
