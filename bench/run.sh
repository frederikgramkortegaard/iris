#!/usr/bin/env bash
# Benchmarks generated code quality: each program is compiled with and
# without the optimization pipeline and timed under wasmtime.
#
# Every benchmark runs under two engine configs:
#   - wasmtime default: cranelift's own optimizer on (real-world numbers)
#   - wasmtime -O opt-level=0: cranelift's optimizer off, which isolates
#     what OUR optimizer contributes instead of letting the JIT hide it
#
# Workloads are sized to hundreds of ms so the ~15ms process startup is
# noise. hyperfine does warmups, ~10 runs and outlier detection.
#
# Needs: hyperfine, wasmtime

set -eu

root="$(cd "$(dirname "$0")/.." && pwd)"
iris="$root/target/release/iris"
out="$root/bench/out"

command -v hyperfine >/dev/null || { echo "hyperfine not found (brew install hyperfine)" >&2; exit 2; }
command -v wasmtime >/dev/null || { echo "wasmtime not found (brew install wasmtime)" >&2; exit 2; }

echo "building release binary..."
cargo build --release --quiet --manifest-path "$root/Cargo.toml"

mkdir -p "$out"
for b in fib mandel nested tail; do
    "$iris" "$root/bench/$b.iris" -o "$out/$b.opt.wat"
    "$iris" "$root/bench/$b.iris" --no-opt -o "$out/$b.noopt.wat"
done

# sanity: both variants must agree before timing anything
check() { # fn args...
    local fn="$1"; shift
    local o n
    o="$(wasmtime run --invoke "$fn" "$out/$fn.opt.wat" "$@" 2>/dev/null)"
    n="$(wasmtime run --invoke "$fn" "$out/$fn.noopt.wat" "$@" 2>/dev/null)"
    if [[ "$o" != "$n" ]]; then
        echo "MISMATCH in $fn: opt=$o noopt=$n" >&2
        exit 1
    fi
}
check fib 30
check mandel 200 200 100
check nested 1000 2 3

bench() { # name fn args...
    local name="$1" fn="$2"; shift 2
    echo
    echo "=== $name ==="
    hyperfine -N --warmup 2 --export-markdown "$out/$fn.md" \
        -n "opt / jit on"      "wasmtime run --invoke $fn $out/$fn.opt.wat $*" \
        -n "no-opt / jit on"   "wasmtime run --invoke $fn $out/$fn.noopt.wat $*" \
        -n "opt / jit off"     "wasmtime run -O opt-level=0 --invoke $fn $out/$fn.opt.wat $*" \
        -n "no-opt / jit off"  "wasmtime run -O opt-level=0 --invoke $fn $out/$fn.noopt.wat $*"
}

bench "fib(40), call-heavy"            fib 40
bench "mandel(1200x1200x500), float-heavy" mandel 1200 1200 500
bench "nested(300000), licm bait"      nested 300000 2 3

echo
echo "=== sum_tail(5000000), recursion depth ==="
echo -n "no-opt: "
wasmtime run --invoke sum_tail "$out/tail.noopt.wat" 5000000 0 2>&1 | grep -m1 -oE "wasm trap.*" || true
echo -n "opt:    "
wasmtime run --invoke sum_tail "$out/tail.opt.wat" 5000000 0 2>/dev/null

echo
echo "markdown tables in bench/out/*.md"
