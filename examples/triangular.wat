(module
  (func $triangular (export "triangular")
    (param $r0 f64)
    (result f64)
    (local $r1 f64)
    (local $r2 f64)
    (local $r3 i32)
    (local $r4 f64)
    (local $r5 f64)
    f64.const 0
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
        f64.add
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