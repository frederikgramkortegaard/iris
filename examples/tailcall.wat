(module
  (func $factorial_tail (export "factorial_tail")
    (param $r0 f64)
    (param $r1 f64)
    (result f64)
    (local $r2 f64)
    (local $r3 f64)
    (local $r4 i32)
    (local $r5 f64)
    (local $r6 f64)
    block
      loop
        local.get $r2
        f64.const 1
        f64.le
        i32.eqz
        br_if 1
        local.get $r2
        f64.const 1
        f64.sub
        local.set $r5
        local.get $r3
        local.get $r2
        f64.mul
        local.set $r6
        local.get $r5
        local.set $r2
        local.get $r6
        local.set $r3
        br 0
      end
    end
    local.get $r3
    return
  )
  (func $sum_tail (export "sum_tail")
    (param $r0 f64)
    (param $r1 f64)
    (result f64)
    (local $r2 f64)
    (local $r3 f64)
    (local $r4 i32)
    (local $r5 f64)
    (local $r6 f64)
    block
      loop
        local.get $r2
        f64.const 0
        f64.le
        i32.eqz
        br_if 1
        local.get $r2
        f64.const 1
        f64.sub
        local.set $r5
        local.get $r3
        local.get $r2
        f64.add
        local.set $r6
        local.get $r5
        local.set $r2
        local.get $r6
        local.set $r3
        br 0
      end
    end
    local.get $r3
    return
  )
)