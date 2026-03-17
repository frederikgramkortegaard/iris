(module
  (func $simple_phi (export "simple_phi")
    (param $r0 f64)
    (param $r1 f64)
    (param $r2 f64)
    (result f64)
    (local $r3 i32)
    (local $r4 f64)
    local.get $r0
    f64.const 0
    f64.gt
    if
      local.get $r1
      local.set $r4
    else
      local.get $r2
      local.set $r4
    end
    unreachable
  )
  (func $critical_edge_case (export "critical_edge_case")
    (param $r0 f64)
    (param $r1 f64)
    (param $r2 f64)
    (param $r3 f64)
    (result f64)
    (local $r4 i32)
    (local $r5 f64)
    (local $r6 i32)
    (local $r7 f64)
    (local $r8 f64)
    (local $r9 i32)
    (local $r10 f64)
    local.get $r0
    f64.const 0
    f64.gt
    if
      local.get $r1
      f64.const 0
      f64.gt
      if
        local.get $r2
        local.set $r10
      else
        local.get $r3
        local.set $r10
      end
      unreachable
    else
      f64.const 0
      local.set $r5
    end
    unreachable
  )
  (func $loop_with_break (export "loop_with_break")
    (param $r0 f64)
    (param $r1 f64)
    (result f64)
    (local $r2 f64)
    (local $r3 f64)
    (local $r4 i32)
    (local $r5 f64)
    (local $r6 i32)
    (local $r7 f64)
    f64.const 0
    local.set $r2
    f64.const 0
    local.set $r3
    local.get $r2
    local.get $r0
    f64.lt
    if
      local.get $r3
      local.get $r2
      f64.add
      local.get $r1
      f64.gt
      if
        local.get $r5
        return
      else
        local.get $r2
        f64.const 1
        f64.add
        local.set $r2
        local.get $r5
        local.set $r3
      end
      unreachable
    else
      local.get $r3
      return
    end
    unreachable
  )
  (func $nested_loop_exit (export "nested_loop_exit")
    (param $r0 f64)
    (param $r1 f64)
    (result f64)
    (local $r2 f64)
    (local $r3 f64)
    (local $r4 i32)
    (local $r5 f64)
    (local $r6 f64)
    (local $r7 f64)
    (local $r8 i32)
    (local $r9 f64)
    (local $r10 f64)
    (local $r11 i32)
    (local $r12 f64)
    (local $r13 f64)
    f64.const 0
    local.set $r2
    f64.const 0
    local.set $r3
    local.get $r2
    local.get $r0
    f64.lt
    if
      local.get $r2
      local.set $r5
      local.get $r3
      local.set $r6
      f64.const 0
      local.get $r1
      f64.lt
      if
        local.get $r6
        f64.const 1
        f64.add
        f64.const 10
        f64.gt
        if
          local.get $r0
          local.set $r12
        else
          local.get $r5
          local.set $r12
        end
        unreachable
      else
        local.get $r5
        f64.const 1
        f64.add
        local.set $r2
        local.get $r6
        local.set $r3
      end
      unreachable
    else
      local.get $r3
      return
    end
    unreachable
  )
  (func $swap_in_loop (export "swap_in_loop")
    (param $r0 f64)
    (param $r1 f64)
    (param $r2 f64)
    (result f64)
    (local $r3 f64)
    (local $r4 f64)
    (local $r5 f64)
    (local $r6 i32)
    (local $r7 f64)
    (local $r8 f64)
    (local $r9 f64)
    local.get $r1
    local.set $r3
    f64.const 0
    local.set $r4
    local.get $r2
    local.set $r5
    block
      loop
        local.get $r4
        local.get $r0
        f64.lt
        i32.eqz
        br_if 1
        local.get $r4
        f64.const 1
        f64.add
        local.set $r7
        local.get $r3
        local.set $r5
        local.get $r7
        local.set $r4
        br 0
      end
    end
    local.get $r3
    local.get $r5
    f64.add
    return
  )
)