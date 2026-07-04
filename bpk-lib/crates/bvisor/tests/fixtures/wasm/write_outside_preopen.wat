(module
  (import "wasi_snapshot_preview1" "path_open"
    (func $path_open
      (param i32 i32 i32 i32 i32 i64 i64 i32 i32)
      (result i32)))
  (import "wasi_snapshot_preview1" "fd_write"
    (func $fd_write (param i32 i32 i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "fd_close"
    (func $fd_close (param i32) (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 16) "../escape.txt")
  (data (i32.const 64) "OUTSIDE-G3-MARKER")
  (func $_start (export "_start")
    i32.const 3
    i32.const 0
    i32.const 16
    i32.const 13
    i32.const 1
    i64.const 64
    i64.const 64
    i32.const 0
    i32.const 0
    call $path_open
    i32.eqz
    if
      i32.const 128
      i32.const 64
      i32.store
      i32.const 132
      i32.const 17
      i32.store
      i32.const 0
      i32.load
      i32.const 128
      i32.const 1
      i32.const 140
      call $fd_write
      drop
      i32.const 0
      i32.load
      call $fd_close
      drop
    end))
