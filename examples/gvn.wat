(module
  (func $main (export "main")
    (param $r0 f64)
    (param $r1 f64)
    (result f64)
    (local $r2 f64)
    (local $r3 f64)
    local.get $r0
    local.get $r1
    f64.add
    local.get $r2
    f64.add
    return
  )
  (func $across_blocks (export "across_blocks")
    (param $r0 f64)
    (param $r1 f64)
    (param $r2 f64)
    (result f64)
    (local $r3 f64)
    (local $r4 i32)
    (local $r5 f64)
    (local $r6 f64)
    local.get $r0
    local.get $r1
    f64.add
    local.set $r3
    local.get $r2
    f64.const 0
    f64.gt
    if
      local.get $r3
      local.get $r3
      f64.add
      return
    else
      local.get $r3
      local.get $r3
      f64.add
      return
    end
    unreachable
  )
)