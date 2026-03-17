(module
  (func $clamp_to_zero (export "clamp_to_zero")
    (param $r0 f64)
    (result f64)
    (local $r1 i32)
    (local $r2 f64)
    local.get $r0
    f64.const 0
    f64.lt
    if
      f64.const 0
      local.set $r2
    else
      local.get $r0
      local.set $r2
    end
    unreachable
  )
  (func $abs_param (export "abs_param")
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
  (func $double_or_zero (export "double_or_zero")
    (param $r0 f64)
    (param $r1 f64)
    (result f64)
    (local $r2 i32)
    (local $r3 f64)
    (local $r4 f64)
    local.get $r1
    f64.const 1
    f64.lt
    if
      f64.const 0
      local.set $r3
    else
      local.get $r0
      local.get $r0
      f64.add
      local.set $r3
    end
    unreachable
  )
)