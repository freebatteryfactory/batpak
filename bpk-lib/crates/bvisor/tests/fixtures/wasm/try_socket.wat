(module
  (import "wasi_snapshot_preview1" "sock_open"
    (func $sock_open (param i32 i32 i32) (result i32)))
  (func $_start (export "_start")
    i32.const 0
    i32.const 0
    i32.const 0
    call $sock_open
    drop))
