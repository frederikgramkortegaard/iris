(module
  (func $main (export "main")
    (result f64)
    (local $r0 f64)
    f64.const 5
    f64.const 10
    f64.add
    return
  )
)