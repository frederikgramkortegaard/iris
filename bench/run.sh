#!/usr/bin/env bash
# Benchmarks generated code quality: each program is compiled with and
# without the optimization pipeline and timed under two engines:
#   - wasmtime: cranelift jit, with its own optimizer on and off
#   - wasmi: pure interpreter, saved instructions show up as saved time
#
# Workloads are sized per engine to hundreds of ms so the ~15ms process
# startup is noise. hyperfine does warmups, ~10 runs and outlier detection.
#
# Needs: hyperfine, wasmtime, wasmi (cargo install wasmi_cli)

set -eu

root="$(cd "$(dirname "$0")/.." && pwd)"
iris="$root/target/release/iris"
out="$root/bench/out"

command -v hyperfine >/dev/null || { echo "hyperfine not found (brew install hyperfine)" >&2; exit 2; }
command -v wasmtime >/dev/null || { echo "wasmtime not found (brew install wasmtime)" >&2; exit 2; }
wasmi="$(command -v wasmi || true)"
[ -n "$wasmi" ] || wasmi="$HOME/.cargo/bin/wasmi"
[ -x "$wasmi" ] || { echo "wasmi not found (cargo install wasmi_cli)" >&2; exit 2; }

echo "building release binary..."
cargo build --release --quiet --manifest-path "$root/Cargo.toml"

mkdir -p "$out"
for b in fib mandel nested tail gvn calls phi orbit; do
    "$iris" "$root/bench/$b.iris" -o "$out/$b.opt.wat"
    "$iris" "$root/bench/$b.iris" --no-opt -o "$out/$b.noopt.wat"
done

# sanity: both variants and both engines must agree before timing anything
check() { # fn args...
    local fn="$1"; shift
    local o n w
    o="$(wasmtime run --invoke "$fn" "$out/$fn.opt.wat" "$@" 2>/dev/null)"
    n="$(wasmtime run --invoke "$fn" "$out/$fn.noopt.wat" "$@" 2>/dev/null)"
    w="$("$wasmi" --invoke "$fn" "$out/$fn.opt.wat" "$@" 2>/dev/null)"
    if [[ "$o" != "$n" || "$o" != "$w" ]]; then
        echo "MISMATCH in $fn: opt=$o noopt=$n wasmi=$w" >&2
        exit 1
    fi
}
check fib 30
check mandel 200 200 100
check nested 1000 2 3
check gvn 100000 1.5 2.5 3.5
check calls 100000
check phi 100000
check orbit 100000 0.001

bench() { # name fn jit_args -- interp_args
    local name="$1" fn="$2"; shift 2
    local jargs=""
    while [[ "$1" != "--" ]]; do jargs="$jargs $1"; shift; done
    shift
    local iargs="$*"
    echo
    echo "=== $name ==="
    hyperfine -N --warmup 2 --export-markdown "$out/$fn.jit.md" \
        -n "opt / jit on"      "wasmtime run --invoke $fn $out/$fn.opt.wat $jargs" \
        -n "no-opt / jit on"   "wasmtime run --invoke $fn $out/$fn.noopt.wat $jargs" \
        -n "opt / jit off"     "wasmtime run -O opt-level=0 --invoke $fn $out/$fn.opt.wat $jargs" \
        -n "no-opt / jit off"  "wasmtime run -O opt-level=0 --invoke $fn $out/$fn.noopt.wat $jargs"
    hyperfine -N --warmup 2 --export-markdown "$out/$fn.interp.md" \
        -n "opt / interp"      "$wasmi --invoke $fn $out/$fn.opt.wat $iargs" \
        -n "no-opt / interp"   "$wasmi --invoke $fn $out/$fn.noopt.wat $iargs"
}

bench "fib, call-heavy"           fib 40 -- 36
bench "mandel, float-heavy"       mandel 1200 1200 500 -- 800 800 100
bench "nested, licm bait"         nested 300000 2 3 -- 100000 2 3
bench "gvn, redundancy bait"      gvn 200000000 1.5 2.5 3.5 -- 40000000 1.5 2.5 3.5
bench "calls, inlining bait"      calls 150000000 -- 30000000
bench "phi, branch-heavy"         phi 500000000 -- 40000000
bench "orbit, whole program"      orbit 10000000 0.001 -- 5000000 0.001

echo
echo "=== sum_tail(5000000), recursion depth ==="
echo -n "no-opt: "
wasmtime run --invoke sum_tail "$out/tail.noopt.wat" 5000000 0 2>&1 | grep -m1 -oE "wasm trap.*" || true
echo -n "opt:    "
wasmtime run --invoke sum_tail "$out/tail.opt.wat" 5000000 0 2>/dev/null

echo
echo "markdown tables in bench/out/*.md"
