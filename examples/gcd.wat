(module
  (func $gcd (export "gcd")
    (param $r0 f64)
    (param $r1 f64)
    (result f64)
    (local $r2 f64)
    (local $r3 f64)
    (local $r4 i32)
    (local $r5 f64)
    (local $r6 f64)
    (local $r7 f64)
    block
      loop
        local.get $r3
        f64.const 0
        f64.eq
        i32.eqz
        br_if 1
        local.get $r2
        local.get $r3
        f64.div
        local.get $r3
        f64.mul
        local.set $r6
        local.get $r2
        local.get $r6
        f64.sub
        local.set $r7
        local.get $r3
        local.set $r2
        local.get $r7
        local.set $r3
        br 0
      end
    end
    local.get $r2
    return
  )
  (func $gcd_iterative (export "gcd_iterative")
    (param $r0 f64)
    (param $r1 f64)
    (result f64)
    (local $r2 f64)
    (local $r3 f64)
    (local $r4 i32)
    (local $r5 f64)
    (local $r6 f64)
    (local $r7 f64)
    local.get $r0
    local.set $r2
    local.get $r1
    local.set $r3
    block
      loop
        local.get $r3
        f64.const 0
        f64.ne
        i32.eqz
        br_if 1
        local.get $r2
        local.get $r3
        f64.div
        local.get $r3
        f64.mul
        local.set $r6
        local.get $r2
        local.get $r6
        f64.sub
        local.set $r7
        local.get $r3
        local.set $r2
        local.get $r7
        local.set $r3
        br 0
      end
    end
    local.get $r2
    return
  )
)