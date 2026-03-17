(module
  (func $licm_simple (export "licm_simple")
    (param $r0 f64)
    (param $r1 f64)
    (param $r2 f64)
    (result f64)
    (local $r3 f64)
    (local $r4 f64)
    (local $r5 i32)
    (local $r6 f64)
    (local $r7 f64)
    (local $r8 f64)
    (local $r9 f64)
    f64.const 0
    local.set $r3
    f64.const 0
    local.set $r4
    local.get $r1
    local.get $r2
    f64.add
    local.set $r7
    block
      loop
        local.get $r3
        local.get $r0
        f64.lt
        i32.eqz
        br_if 1
        local.get $r4
        local.get $r7
        f64.add
        local.set $r8
        local.get $r3
        f64.const 1
        f64.add
        local.set $r3
        local.get $r8
        local.set $r4
        br 0
      end
    end
    local.get $r4
    return
  )
  (func $licm_chain (export "licm_chain")
    (param $r0 f64)
    (param $r1 f64)
    (param $r2 f64)
    (result f64)
    (local $r3 f64)
    (local $r4 f64)
    (local $r5 i32)
    (local $r6 f64)
    (local $r7 f64)
    (local $r8 f64)
    (local $r9 f64)
    (local $r10 f64)
    f64.const 0
    local.set $r3
    f64.const 0
    local.set $r4
    local.get $r1
    local.get $r2
    f64.add
    f64.const 2
    f64.mul
    local.set $r8
    block
      loop
        local.get $r3
        local.get $r0
        f64.lt
        i32.eqz
        br_if 1
        local.get $r4
        local.get $r8
        f64.add
        local.set $r9
        local.get $r3
        f64.const 1
        f64.add
        local.set $r3
        local.get $r9
        local.set $r4
        br 0
      end
    end
    local.get $r4
    return
  )
  (func $licm_nested (export "licm_nested")
    (param $r0 f64)
    (param $r1 f64)
    (param $r2 f64)
    (result f64)
    (local $r3 f64)
    (local $r4 f64)
    (local $r5 i32)
    (local $r6 f64)
    (local $r7 f64)
    (local $r8 f64)
    (local $r9 i32)
    (local $r10 f64)
    (local $r11 f64)
    (local $r12 f64)
    (local $r13 f64)
    (local $r14 f64)
    f64.const 0
    local.set $r3
    f64.const 0
    local.set $r4
    local.get $r1
    local.get $r2
    f64.mul
    f64.const 1
    f64.add
    local.set $r11
    local.get $r4
    local.get $r0
    f64.lt
    if
      local.get $r3
      local.set $r7
      f64.const 0
      local.set $r8
      block
        loop
          local.get $r8
          local.get $r0
          f64.lt
          i32.eqz
          br_if 1
          local.get $r7
          local.get $r11
          f64.add
          local.set $r12
          local.get $r8
          f64.const 1
          f64.add
          local.set $r13
          local.get $r12
          local.set $r7
          local.get $r13
          local.set $r8
          br 0
        end
      end
      local.get $r4
      f64.const 1
      f64.add
      local.set $r14
      local.get $r7
      local.set $r3
      local.get $r14
      local.set $r4
    else
      local.get $r3
      return
    end
    unreachable
  )
)