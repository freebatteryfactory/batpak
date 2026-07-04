;; Anti-vacuous socket-denial proof (NOT an unknown-import failure).
;;
;; The guest imports only functions this backend actually provides
;; (`fd_write` + preview1 `sock_recv`), so it LINKS cleanly. It then prints a
;; 13-byte marker (proving it ran) and attempts a real `sock_recv` on fd 0 — a
;; descriptor that is NOT a socket (no socket capability was installed). The
;; call MUST fail (EBADF/ENOTSOCK). If it instead SUCCEEDS (errno == 0), a
;; socket was reachable — the guest traps via `unreachable`, flipping the run
;; to a non-Completed outcome so the test reds. The denial is thus WITNESSED by
;; the guest, not asserted by a printed sticker.
(module
  (import "wasi_snapshot_preview1" "fd_write"
    (func $fd_write (param i32 i32 i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "sock_recv"
    (func $sock_recv (param i32 i32 i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 64) "NO-SOCKET-CAP")
  (func $_start (export "_start")
    ;; Print the 13-byte marker (iovec at 128 -> base 64, len 13; nwritten 140).
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
    drop
    ;; Recv iovec at 16 -> base 200, len 16.
    i32.const 16
    i32.const 200
    i32.store
    i32.const 20
    i32.const 16
    i32.store
    ;; sock_recv(fd=0, ri_data=16, ri_data_len=1, ri_flags=0, ro_datalen=24, ro_flags=28).
    i32.const 0
    i32.const 16
    i32.const 1
    i32.const 0
    i32.const 24
    i32.const 28
    call $sock_recv
    ;; errno == 0 means a socket was reachable (leak) -> trap so the test reds.
    i32.eqz
    if
      unreachable
    end))
