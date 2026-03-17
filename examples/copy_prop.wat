(module
  (func $main (export "main")
    (param $r0 f64)
    (param $r1 f64)
    (result f64)
    (local $r2 f64)
    local.get $r0
    local.get $r0
    f64.add
    return
  )
  (func $chain (export "chain")
    (param $r0 f64)
    (result f64)
    local.get $r0
    return
  )
)