(module
  (func $power (export "power")
    (param $r0 f64)
    (param $r1 f64)
    (result f64)
    (local $r2 i32)
    (local $r3 f64)
    (local $r4 f64)
    (local $r5 f64)
    local.get $r1
    f64.const 0
    f64.eq
    if
      f64.const 1
      return
    else
      local.get $r1
      f64.const 1
      f64.sub
      local.set $r3
      local.get $r0
      local.get $r3
      call $power
      local.set $r4
      local.get $r0
      local.get $r4
      f64.mul
      return
    end
    unreachable
  )
  (func $power_iterative (export "power_iterative")
    (param $r0 f64)
    (param $r1 f64)
    (result f64)
    (local $r2 f64)
    (local $r3 f64)
    (local $r4 i32)
    (local $r5 f64)
    (local $r6 f64)
    f64.const 0
    local.set $r2
    f64.const 1
    local.set $r3
    block
      loop
        local.get $r2
        local.get $r1
        f64.lt
        i32.eqz
        br_if 1
        local.get $r3
        local.get $r0
        f64.mul
        local.set $r5
        local.get $r2
        f64.const 1
        f64.add
        local.set $r2
        local.get $r5
        local.set $r3
        br 0
      end
    end
    local.get $r3
    return
  )
)