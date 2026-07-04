(module
  (import "wasi_snapshot_preview1" "environ_sizes_get"
    (func $environ_sizes_get (param i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "environ_get"
    (func $environ_get (param i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "fd_write"
    (func $fd_write (param i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 512) "ENV-GRID-MARKER")
  (func $write_marker
    i32.const 600
    i32.const 512
    i32.store
    i32.const 604
    i32.const 15
    i32.store
    i32.const 1
    i32.const 600
    i32.const 1
    i32.const 620
    call $fd_write
    drop)
  (func $_start (export "_start")
    (local $ptr i32)
    (local $ok i32)
    i32.const 0
    i32.const 4
    call $environ_sizes_get
    i32.eqz
    if
      i32.const 0
      i32.load
      i32.const 0
      i32.gt_u
      if
        i32.const 128
        i32.const 256
        call $environ_get
        i32.eqz
        if
          i32.const 128
          i32.load
          local.set $ptr
          i32.const 1
          local.set $ok
          local.get $ptr i32.load8_u i32.const 66 i32.ne if i32.const 0 local.set $ok end
          local.get $ptr i32.const 1 i32.add i32.load8_u i32.const 86 i32.ne if i32.const 0 local.set $ok end
          local.get $ptr i32.const 2 i32.add i32.load8_u i32.const 95 i32.ne if i32.const 0 local.set $ok end
          local.get $ptr i32.const 3 i32.add i32.load8_u i32.const 71 i32.ne if i32.const 0 local.set $ok end
          local.get $ptr i32.const 4 i32.add i32.load8_u i32.const 82 i32.ne if i32.const 0 local.set $ok end
          local.get $ptr i32.const 5 i32.add i32.load8_u i32.const 73 i32.ne if i32.const 0 local.set $ok end
          local.get $ptr i32.const 6 i32.add i32.load8_u i32.const 68 i32.ne if i32.const 0 local.set $ok end
          local.get $ptr i32.const 7 i32.add i32.load8_u i32.const 61 i32.ne if i32.const 0 local.set $ok end
          local.get $ptr i32.const 8 i32.add i32.load8_u i32.const 69 i32.ne if i32.const 0 local.set $ok end
          local.get $ptr i32.const 9 i32.add i32.load8_u i32.const 88 i32.ne if i32.const 0 local.set $ok end
          local.get $ptr i32.const 10 i32.add i32.load8_u i32.const 80 i32.ne if i32.const 0 local.set $ok end
          local.get $ptr i32.const 11 i32.add i32.load8_u i32.const 69 i32.ne if i32.const 0 local.set $ok end
          local.get $ptr i32.const 12 i32.add i32.load8_u i32.const 67 i32.ne if i32.const 0 local.set $ok end
          local.get $ptr i32.const 13 i32.add i32.load8_u i32.const 84 i32.ne if i32.const 0 local.set $ok end
          local.get $ptr i32.const 14 i32.add i32.load8_u i32.const 69 i32.ne if i32.const 0 local.set $ok end
          local.get $ptr i32.const 15 i32.add i32.load8_u i32.const 68 i32.ne if i32.const 0 local.set $ok end
          local.get $ok
          if
            call $write_marker
          end
        end
      end
    end))
