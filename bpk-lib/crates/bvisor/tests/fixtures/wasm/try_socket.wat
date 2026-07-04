;; DenyAll smoke guest for the network-absence proof.
;;
;; NOTE: this guest does NOT (and cannot) witness network denial itself.
;; wasi_snapshot_preview1 exposes no sock_open/sock_connect, so a guest can never
;; originate a socket regardless of policy — a guest probe of any fd is denied by
;; fd-typing, not by network confinement, and a leaked socket would sit at an fd the
;; guest never names. The genuine, ANTI-VACUOUS proof that no network capability is
;; installed lives in the test body (an AllowList request must be REFUSED) and in
;; coupling_proof_wasm (NetworkAllowList must never enter the ceiling). This guest
;; only proves the DenyAll admission path runs to completion.
(module
  (import "wasi_snapshot_preview1" "fd_write"
    (func $fd_write (param i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 64) "NO-SOCKET-CAP")
  (func $_start (export "_start")
    i32.const 128
    i32.const 64
    i32.store
    i32.const 132
    i32.const 13
    i32.store
    i32.const 1
    i32.const 128
    i32.const 1
    i32.const 140
    call $fd_write
    drop))
