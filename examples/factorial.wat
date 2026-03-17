(module
  (func $factorial (export "factorial")
    (param $r0 f64)
    (result f64)
    (local $r1 i32)
    (local $r2 f64)
    (local $r3 f64)
    (local $r4 f64)
    local.get $r0
    f64.const 1
    f64.le
    if
      f64.const 1
      return
    else
      local.get $r0
      f64.const 1
      f64.sub
      call $factorial
      local.set $r3
      local.get $r0
      local.get $r3
      f64.mul
      return
    end
    unreachable
  )
  (func $factorial_iterative (export "factorial_iterative")
    (param $r0 f64)
    (result f64)
    (local $r1 f64)
    (local $r2 f64)
    (local $r3 i32)
    (local $r4 f64)
    (local $r5 f64)
    f64.const 1
    local.set $r1
    f64.const 1
    local.set $r2
    block
      loop
        local.get $r2
        local.get $r0
        f64.le
        i32.eqz
        br_if 1
        local.get $r1
        local.get $r2
        f64.mul
        local.set $r4
        local.get $r2
        f64.const 1
        f64.add
        local.set $r5
        local.get $r4
        local.set $r1
        local.get $r5
        local.set $r2
        br 0
      end
    end
    local.get $r1
    return
  )
)