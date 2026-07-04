(module
  (import "wasi_snapshot_preview1" "fd_write"
    (func $fd_write (param i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 64) "CAPTURE-STREAM-MARKER")
  (func $_start (export "_start")
    i32.const 128
    i32.const 64
    i32.store
    i32.const 132
    i32.const 21
    i32.store
    i32.const 1
    i32.const 128
    i32.const 1
    i32.const 140
    call $fd_write
    drop))
