(module
  (func $sign (export "sign")
    (param $r0 f64)
    (result f64)
    (local $r1 i32)
    (local $r2 f64)
    (local $r3 i32)
    (local $r4 f64)
    local.get $r0
    f64.const 0
    f64.lt
    if
      f64.const -1
      local.set $r2
    else
      local.get $r0
      f64.const 0
      f64.gt
      if
        f64.const 1
        local.set $r4
      else
        f64.const 0
        local.set $r4
      end
      unreachable
    end
    unreachable
  )
  (func $abs_v2 (export "abs_v2")
    (param $r0 f64)
    (result f64)
    (local $r1 i32)
    (local $r2 f64)
    (local $r3 f64)
    local.get $r0
    f64.const 0
    f64.lt
    if
      f64.const 0
      local.get $r0
      f64.sub
      local.set $r3
    else
      local.get $r0
      local.set $r3
    end
    unreachable
  )
  (func $clamp_v2 (export "clamp_v2")
    (param $r0 f64)
    (param $r1 f64)
    (param $r2 f64)
    (result f64)
    (local $r3 i32)
    (local $r4 i32)
    (local $r5 f64)
    (local $r6 f64)
    local.get $r0
    local.get $r1
    f64.lt
    if
      local.get $r1
      local.set $r6
    else
      local.get $r0
      local.get $r2
      f64.gt
      if
        local.get $r2
        local.set $r5
      else
        local.get $r0
        local.set $r5
      end
      unreachable
    end
    unreachable
  )
  (func $fib_step (export "fib_step")
    (param $r0 f64)
    (param $r1 f64)
    (param $r2 f64)
    (result f64)
    (local $r3 i32)
    (local $r4 f64)
    (local $r5 f64)
    local.get $r2
    f64.const 1
    f64.lt
    if
      local.get $r0
      local.set $r5
    else
      local.get $r0
      local.get $r1
      f64.add
      local.set $r5
    end
    unreachable
  )
)