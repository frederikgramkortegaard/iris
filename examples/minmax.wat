(module
  (func $min (export "min")
    (param $r0 f64)
    (param $r1 f64)
    (result f64)
    (local $r2 i32)
    local.get $r0
    local.get $r1
    f64.lt
    if
      local.get $r0
      return
    else
      local.get $r1
      return
    end
    unreachable
  )
  (func $max (export "max")
    (param $r0 f64)
    (param $r1 f64)
    (result f64)
    (local $r2 i32)
    local.get $r0
    local.get $r1
    f64.gt
    if
      local.get $r0
      return
    else
      local.get $r1
      return
    end
    unreachable
  )
  (func $abs (export "abs")
    (param $r0 f64)
    (result f64)
    (local $r1 i32)
    (local $r2 f64)
    local.get $r0
    f64.const 0
    f64.lt
    if
      f64.const 0
      local.get $r0
      f64.sub
      return
    else
      local.get $r0
      return
    end
    unreachable
  )
  (func $clamp (export "clamp")
    (param $r0 f64)
    (param $r1 f64)
    (param $r2 f64)
    (result f64)
    (local $r3 i32)
    (local $r4 i32)
    local.get $r0
    local.get $r1
    f64.lt
    if
      local.get $r1
      return
    else
      local.get $r0
      local.get $r2
      f64.gt
      if
        local.get $r2
        return
      else
        local.get $r0
        return
      end
      unreachable
    end
    unreachable
  )
)