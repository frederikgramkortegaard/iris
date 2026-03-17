(module
  (func $fibonacci (export "fibonacci")
    (param $r0 f64)
    (result f64)
    (local $r1 i32)
    (local $r2 f64)
    (local $r3 f64)
    (local $r4 f64)
    (local $r5 f64)
    (local $r6 f64)
    local.get $r0
    f64.const 1
    f64.le
    if
      local.get $r0
      return
    else
      local.get $r0
      f64.const 1
      f64.sub
      call $fibonacci
      local.set $r3
      local.get $r0
      f64.const 2
      f64.sub
      call $fibonacci
      local.set $r5
      local.get $r3
      local.get $r5
      f64.add
      return
    end
    unreachable
  )
)