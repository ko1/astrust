# saruby Thread / blocking I/O — the blop layer

Every place a green thread waits goes through one suspension point, `thread::wait(op)`, and the
interpreter thread sleeps in exactly one place, the engine's pump. This is koruby_precise's
**blop** (BLocking OPeration) design (`~/ruby/astro/sample/koruby_precise/docs/io_design.md`),
rebuilt for Rust: the engine owns in-flight operations instead of pointing into the parked thread's
stack. Code: `src/builtins/blop.rs` (slab, engines), `src/builtins/thread.rs` (scheduler, `wait`,
`block_on`, `poll_wait`, `read_op`, `write_op`), FFI in `src/builtins/sys.rs`.
Measurements: `~/ruby/src/trials/2026-11-22-saruby-blop/`.

Before this, a waiting thread was a condition closure polled by the scheduler: each pass switched
into every waiting thread to re-check it, `poll(fd, 0)` stood in for readiness, and when any thread
waited on a fd the scheduler slept 1 ms and polled again (≈940 wake-ups per second while idle, and
I/O latency in 1 ms steps: a pipe round trip through `cat` ran at ~1,100/s).

## Mapping to koruby

| koruby (C) | saruby (Rust) | why it differs |
|---|---|---|
| `struct korb_blop` on the parked thread's C stack; the engine keeps `&blop` across the switch | `blop::Op` moved into a slab owned by the engine; the thread keeps an `Id` (index + generation) and takes `(Op, Res)` back on resume | a borrow cannot be followed across a fiber switch; a pointer into another fiber's stack would need `unsafe` and a lifetime nobody checks. An Id is plain data: a stale one never names a reused slot (generation check), and it is not a GC root |
| `u.rw.buf` must be stable memory (malloc / C stack), a convention checked by CodeQL | `Op::Read { buf: Vec<u8> }` / `Op::Write { buf }`: the buffer is **owned** by the op and handed back with the result (tokio-uring / monoio style) | ownership is the stable-buffer rule, enforced by the type system: nothing else can touch or free the buffer while the op is in flight. io_uring needs exactly this (the kernel may write until the CQE, even after a cancel) |
| `kind` enum + union | `enum Op { Poll(Vec<PollFd>), Read, Write, Timer, Wake(u64) }` | `Wake` is new (below) |
| `result`: `ssize_t`, negative = -errno | `Res = Result<usize, i64>`: `Err(ETIMEDOUT)` deadline, `Err(ECANCELED)` interrupt / wakeup, `Err(EBADF)` fd closed by another thread | |
| engine vtable (`init/prep/cancel/pump`) | `trait Engine { submit, retire, pump, after_fork }` | readiness engines report `Ready { id, idx, revents }`; `Slab::apply_ready` does the I/O for Read / Write (completion emulated in one place, shared by poll and epoll). A completion engine would call `Slab::complete` itself |
| `korb_blop_post / cancel / cancel_fd` | `Slab::complete`, `blop::interrupt(id, errno)`, `blop::cancel_fd(fd)` | |
| `korb_blop_wait` (check_ints on return) | `thread::wait(c, op, deadline, deadlockable)`; pending Thread#raise / #kill delivered on return; completion wins a race with an interrupt (its result is dropped, as in CRuby) | |
| Mutex / CV / Queue: explicit wait queues of `korb_thread*`, plain park | the same waits are **blops** of kind `Wake(key)` (key = the object's address; SizedQueue pushers use address\|1); `notify_one(key)` completes the oldest waiter (Mutex unlock, Queue push, SizedQueue pop), `notify(key)` all of them (join, close, CV broadcast); a woken waiter that is interrupted instead passes its wake-up on | one park mechanism, one runnable test (`op done or interrupted`), one deadlock rule. The wait list (`Vec<(key, Id)>`) is in the blop state, not in the objects (GC payloads; the slab holds no Values) |
| run queue, any thread can be the scheduler | the main thread's loop (`thread::schedule`) switches in the **runnable** threads (unstarted, yielded, op done, or interrupted) | saruby's threads are fiber chains entered from the main thread (unchanged). A parked thread is no longer switched in to re-check anything |
| eventfd registered permanently, signals wake the pump | the same: `on_signal` writes the eventfd (`blop::signal_wake`); trap handlers run in the main thread's loop while others are parked | |
| probe uring → epoll → poll | `SARUBY_BLOP_ENGINE=poll` (default) / `epoll` | measured (below): with a few fds poll is faster |

## The pieces

**Where the state lives**: `Vm::blop` (`Option<Box<Blop>>`, made at the first wait), reached through
the `Ctx`. A first version kept it in a `thread_local!` `RefCell`: in `libsaruby.so` every access
is a `__tls_get_addr` call plus a borrow flag, 16–20 % of the instructions of
`vm_thread_condvar1` / `sized_queue4`. The signal handler only touches an atomic (the eventfd
number). An IO collected by the GC needs no cancel: a thread waiting on it roots it.

**Slab** (`blop.rs`): `Vec<Entry { gen, slot }>` with a free list; slot = `Free | Pending { op, deadline, deadlockable } | Done(op, res)`.
`submit` inserts and registers interest; completion moves `Pending → Done` once (later posts are
no-ops); `take` releases the slot (bumping the generation) and returns the op — withdrawing it with
`Err(ECANCELED)` if it had not completed. Deadlines are generic (`expire`): a `Timer` completes
`Ok(0)`, anything else `Err(ETIMEDOUT)`. Interest is retired as soon as an op completes (a
level-triggered epoll fd would otherwise keep firing).

**wait / park / schedule** (`thread.rs`): a non-main thread records `blop = Some(id)` and switches
out to the main thread; `run_pass` resumes it only when `blop::is_done(id)` or it has a pending
interrupt. The main thread's `wait` runs the loop itself:

```
loop {
  done or interrupted → return
  run trap handlers (may raise: the op is withdrawn and the exception propagates)
  run_pass: each runnable thread once → if any ran: pump(non-blocking), continue
  nothing runnable and nothing can complete alone → fatal "No live threads left. Deadlock?"
  pump(blocking)      ← the only place the interpreter thread sleeps
}
```

"Can complete alone" = some pending op waits on a fd, has a deadline, or is not deadlockable
(`sleep` without a duration is not deadlockable, as in CRuby 4.0: `Thread.new{sleep}; sleep` hangs,
`Queue.new.pop` in the only other thread is a deadlock).

**pump(block)**: expire deadlines (the clock is read only if some op has a deadline —
`clock_gettime` is a real syscall under the hpet clocksource of the dev box: 40,000 of them made a
Queue ping-pong 7x slower), ask the engine for readiness with timeout = 0 or the nearest deadline,
apply it, expire again. Read/Write: at most one per fd per pump (a second read of a blocking fd
after the first consumed the data would block; the next pump polls again).

**Interrupts**: `Thread#raise / #kill / #wakeup / #run` set the flag and `blop::interrupt(id,
ECANCELED)` the target's op, so the scheduler resumes it; `wait` then raises (raise / kill) or the
caller loops (wakeup: `sleep` returns, `Queue#pop` re-checks and waits again).

**IO#close** (`io.rs close_fds` and close_read / close_write): `blop::cancel_fd(fd)` before
`close(2)`, so a thread waiting on that fd gets `IOError: stream closed in another thread` and the
fd number cannot be reused under an op (and is still open when epoll deregisters it).

**fork**: in the child only the forking thread survives (`thread::after_fork_child`): the other
threads are marked dead, the slab is cleared, the eventfd and the epoll instance (both shared with
the parent) are replaced. A child forked from a non-main thread makes that thread its main thread
and exits when it ends (`Thread.current == Thread.main` in the child, as CRuby).

## Call sites

| Ruby | op |
|---|---|
| `Kernel#sleep` (with or without other threads), `Thread.stop`, `Mutex#sleep` | `Timer` (deadline or none) |
| `Thread#join(t)` / `#value` | `Wake(thread)`, notified at thread end |
| `Mutex#lock` / `Monitor#enter`, `ConditionVariable#wait`, `Queue#pop`, `SizedQueue#push` | `Wake(mutex / cv token / queue)` with the timeout as deadline |
| IO reads with an empty buffer (`read`, `readpartial`, `gets`, `getc`, `each_line`, …) on a fd that is not ready | `Read` into a 64 KiB owned buffer, appended to the IO's read buffer |
| buffered writes / flush to a fd that is not writable | `Write` of ≤ 4096 bytes (POLLOUT promises PIPE_BUF) |
| `IO.select`, `IO#wait_readable / wait_writable / wait`, `#sysread` | `Poll` (all fds, with the timeout) |
| `Process.wait(pid)`, `system`, `` ` ``, `IO#close` of a popen | `Poll` on a pidfd (`pidfd_open`), then `waitpid(WNOHANG)` |
| socket `accept`, `connect` (non-blocking connect, then SO_ERROR), `recv` / `recvfrom` / `recvmsg`, `send` / `sendmsg` on a full buffer (`builtins/socket.rs`); sockets are non-blocking fds, as in CRuby, and `IO#timeout` is the deadline | `Poll` on the socket fd, then the call is retried |
| `getaddrinfo` / reverse `getnameinfo` while other threads exist | a `std::thread` worker runs the call (`Send + 'static` closure); the waiting thread polls an eventfd the worker writes (`socket.rs offload`) |
| `Process.wait` of any child / a group, `File#flock` that would block, `File.open` of a FIFO writer without a reader | retry after a back-off `Timer` (1 ms doubling to 20 / 50 ms): no fd to wait on. A CFUNC worker (below) is the exact form |

Without other threads, plain reads and writes still block in the syscall (nothing else could run);
`sleep`, `IO.select` and `#wait*` always use the blop path: the main loop with no threads is one
ppoll that also wakes for signals through the eventfd. (The old `nanosleep` loop could miss a signal
that arrived between its pending check and the call: a forked child of rubyspec process/wait slept
through its TERM.) As in CRuby 4.0, a trap handler runs and the sleep goes on, with or without a
duration (`Mutex#sleep` forever counts for deadlock, `Kernel#sleep` forever does not).

## Engines

**poll** (default): no registration; each pump builds the pollfd array from the slab (wake fd first)
and calls `ppoll` with a nanosecond timeout.

**epoll**: one registration per fd carrying the union of its ops' events (level-triggered);
`EPOLL_CTL_ADD/MOD/DEL` on submit / retire. Regular files and directories (`EPERM`) are reported
ready at once, as poll(2) does. `epoll_pwait2` gives a nanosecond timeout (probed at open; without
it the engine falls back to poll). After fork the instance is recreated.
Cost: two `epoll_ctl` per op. In the two-thread pipe ping-pong (2 fds) that is 80k round trips/s
against poll's 124k, which is why poll is the default. epoll pays off only with many fds waited on
at once; keeping registrations across ops (a registration cache keyed by fd, deregistered on
`cancel_fd`) would remove most of the cost and is the next step for it.
Known hole: a fd closed with `close(2)` that bypasses `IO#close` while an op waits on it is silently
dropped from the epoll set (poll would report POLLNVAL).

**io_uring** (design only). It is the reason the interface is completion-shaped:
- `submit`: Read / Write become `IORING_OP_READ / WRITE` SQEs pointing into the op's `Vec` buffer;
  Poll becomes `IORING_OP_POLL_ADD`; deadlines stay in the slab (or `IORING_OP_LINK_TIMEOUT`).
  Regular-file reads become truly asynchronous (the readiness engines block the interpreter on a
  slow disk, the one limitation readiness cannot fix).
- `pump`: `io_uring_enter(min_complete = 1, timeout)`; each CQE → `Slab::complete(id, res)`.
- cancel: `IORING_OP_ASYNC_CANCEL`; until the CQE arrives the kernel may still write into the
  buffer, so a withdrawn op must stay in the slab as an `Orphan` slot owning its buffer (the slot
  is freed, and the buffer dropped, when the CQE comes). Rust ownership makes this explicit: the
  buffer cannot be handed back to the caller while the kernel holds it. "Completion wins" falls out:
  a CQE with a result that races the cancel is delivered and the interrupt raised after it.
- `unsafe` needed: the ring mmap, SQE writes, and the buffer address given to the kernel — about the
  same narrow set of `sys.rs` wrappers, plus the invariant "a Vec whose pointer is in an SQE is not
  moved/reallocated/dropped until its CQE" held by the slab (a `Pin<Box<[u8]>>` buffer type would
  state it in the type).
- probe order uring → (ENOSYS / EPERM: container seccomp) → epoll → poll, as koruby.

**CFUNC** (one ad-hoc form: the resolver worker of socket.rs): koruby's worker-thread kind for calls that cannot be multiplexed
(`getaddrinfo`, blocking `open` of a FIFO, `flock`). In Rust the "fn must not touch the VM"
contract is a `Send + 'static` bound on the closure — `Ctx` and heap borrows are not `Send`, so the
compiler checks what koruby needs a CodeQL rule for. The back-off emulations above are its
placeholders.

## Limits

1. No preemption (unchanged): a thread that never blocks (`loop { m.synchronize {} }`, a busy
   `while` in the main thread) starves the others. rubyspec core/mutex/lock and core/thread/list
   still time out on exactly this; it needs a time-slice check at loop back-edges / method entry
   (koruby's timer thread + `korb_loop_wants_yield`), which is a node.def change left for later.
2. Regular-file disk waits block the interpreter (poll / epoll); io_uring would fix it.
3. `IO.select` on a fd another thread closes raises IOError here; CRuby 4.0.2 hangs in that case.
4. Readiness writes are ≤ 4096 bytes per wake-up (a socket's POLLOUT promises less than a pipe's).
5. The FIFO open emulation lets a reader return from `open` before a writer connects (its first read
   waits instead).
6. `Kernel#require` of a feature another thread is loading does not wait for it (rubyspec
   kernel/require "(concurrently)"; it also needs `Thread#backtrace` of another thread).

## Encoding conversion

`src/builtins/io_conv.rs` is io.c's conversion layer on the engine of `econv.rs`
(measurements and the differential corpus: `~/ruby/src/trials/2026-11-23-saruby-io-transcode/`).

| io.c | saruby | note |
|---|---|---|
| `encs.enc / enc2 / ecflags / ecopts` | `RIo.enc / enc2 / ecflags` (CRuby's ECONV_* bits) `/ ecrep` (the `replace:` string, boxed) | `ecflags` and `replace:` come from `transcode::prepare_options`, the parser String#encode uses |
| `NEED_READCONV` (Unix) | `need_readconv`: an internal encoding, or a newline decorator (`"rt"`, `newline:`, `universal_newline:`) | text mode is no separate flag: it is "a newline decorator and not binmode", what `validate_enc_binmode` leaves in `FMODE_TEXTMODE` |
| `readconv`, `cbuf` (capa ≥ 128 KiB) | `IoConv { rc, cbuf, cpos, ccap }`, boxed in `RIo.conv`, made on first use | a stream that never converts carries one `None` |
| `fill_cbuf` / `more_char` (`ECONV_AFTER_OUTPUT`) | the same, with putback into rbuf and the flush at EOF | AFTER_OUTPUT is kept: after `gets` the bytes past the line are still unconverted in rbuf, so `read(n)` / `pos` / `set_encoding` see what CRuby's do |
| `appendline` / `swallow` / `rb_io_getline_0`, `io_getc` | `getline::<S>`, `swallow::<S>`, `getc` / `getc_raw` over a `Src` (`Raw` = rbuf, `Conv` = cbuf) | the general branch serves both (a limit relaxed for a cut character, paragraph mode, multibyte separators, no lineno for a line cut by the limit); the fast path (`"\n"`, no limit, ASCII-compatible, no converter) is the old `read_line` |
| `rb_io_check_byte_readable` | `byte_readable` / `no_char_pending` | `read(n)`, `readpartial`, `read_nonblock`, `sysread`, `getbyte`, `ungetbyte`: "byte oriented read for character buffered IO" |
| `rb_io_ungetc` | converted streams push into cbuf ("ungetc failed" past its capacity), others into rbuf with CRuby's capacity rule ("ungetbyte failed") | `RIo.rcap` is the capacity CRuby's rbuf would have (8 KiB, 128 KiB with a converter) |
| `make_writeconv` / `do_writeconv` | `make_writeconv` / `write_plan` + `transcode::str_encode` (rb_str_encode) + `writeconv_convert` | ASCII-incompatible externals (UTF-16/32) and stateful decorators get the double conversion; `finish_writeconv` writes the closing bytes (ISO-2022-JP's `ESC ( B`) at close and at finalization |
| `clear_codeconv` | `set_encoding`, `binmode`, `reopen`, `seek` / `pos` with unread bytes; `rewind` drops the readconv | as CRuby, `set_encoding` drops the writeconv unfinished |

With other threads running, a step that would block hands back what it took (to rbuf or cbuf),
waits through the blop layer (`coop_fill`) and runs again (`src_run`); a character cut at the buffer
end is resumed where it was (`getc_raw`'s carry), so the result does not depend on how the bytes
arrived.

Not as in CRuby: GB18030 / Big5 / EUC-KR find the start of the last character of a limit cut by a
forward scan (CRuby has a `left_adjust_char_head` per encoding; UTF-8/16/32, EUC-JP, Shift_JIS and
the single-byte ones follow CRuby's).  File reads take 64 KiB at a time where CRuby takes 8 KiB, so
CRuby's quirk of a limit cut inside a UTF-16 newline swallowing the rest of its 8 KiB buffer gives a
longer line here.

## IO::Buffer

`src/builtins/io_buffer.rs` (`forbid(unsafe_code)`) is io_buffer.c of CRuby 4.0.2, loaded on first
reference of `IO::Buffer` (an internal autoload of `saruby/io_buffer`; startup does not change).
Measurements, the differential scripts and the cases where 4.0.2 crashes:
`~/ruby/src/trials/2026-11-23-saruby-io-buffer/`.

**Where the bytes are.** A buffer that owns memory holds it as a `Vec<u8>` (INTERNAL) or a
`sys::Mapping` (MAPPED: anonymous, or a file for `IO::Buffer.map`), which unmaps on drop. A buffer of
`IO::Buffer.for` / `.string` owns nothing: its bytes are the String's own buffer. A slice owns
nothing either. Every buffer keeps what CRuby's struct keeps: `base` (an address), `size`, `flags`
and `source` (nil, the String, or the root buffer of a slice; a GC-traced Value).

**Validity without dangling pointers.** CRuby validates a slice by checking that `[base, base+size)`
still lies inside its source's current memory (`io_buffer_validate_slice`), then dereferences `base`.
saruby does the same check (`at`), but `base` is only compared, never dereferenced: the bytes are
taken from the source's current `Vec` / mapping / String with a bounds-checked `get(off..off+size)`,
`off = base - source.base`. So `free`, `transfer` and `resize` invalidate slices exactly when 4.0.2
does (a resize that keeps the address keeps them valid; a realloc that moves the memory invalidates
them), and no outcome of that check can reach memory the source does not own now: at worst a slice
reads the source's current bytes at the same offset, as CRuby does when a new allocation lands on the
old address. The GC keeps the source alive while a slice names it; no reference count is needed.

**Re-entry.** Every method takes its borrow of the bytes after the arguments are converted, and takes
it again after anything that can run Ruby code (`to_int` / `to_f` in `set_value`, the block of `each`,
a thread switch while `read` / `write` wait). Where 4.0.2 keeps the pointer it took first and writes
into freed memory (a `to_int` that frees the buffer, an `each` block that frees it), saruby raises
the error the new state calls for (`Type extends beyond end of buffer!`, `InvalidatedError`). Other
deliberate differences: `each_byte` with a count past the end raises instead of reading past it;
`and!` / `or!` / `xor!` with an invalid mask raise instead of reading freed memory; `slice.resize(0)`
gives a null buffer instead of a copy of uninitialized memory; growing a private file mapping past the
end of the file copies it into anonymous memory instead of faulting (SIGBUS) on the new tail.

**Blocking.** `read` / `write` / `pread` / `pwrite` lock the buffer for the call (as CRuby's blocking
region) and loop over the system call until `length` bytes. With other threads, a blocking fd is
first waited on through the blop layer (`io::wait_one`), a write takes at most PIPE_BUF bytes per
wake-up (POLLOUT promises no more), and the bytes are resolved again after each wait. A nonblocking
fd returns `-EAGAIN` as in CRuby. CRuby's `IO.pipe` fds are O_NONBLOCK; saruby's are not, so the same
program blocks here where CRuby gets `-11` (a difference of `IO.pipe`, not of IO::Buffer).

**Strings.** `IO::Buffer.for(str) { }` and `.string` lock the String (`FL_TMPLOCK`, rb_str_locktmp):
`str_modify`, `check_frozen` and `String#freeze` raise "can't modify string; temporarily locked".
Not covered: the String#<< fast path of `node_opt_ltlt` (node.def) appends to a locked String.

**Unsafe.** Five `unsafe` in `sys.rs` (`Mapping::new` mmap, `remap` mremap, `Drop` munmap — the three
through the imported `syscall(2)`, no new libc symbol — and the slice views `bytes`, `bytes_mut`);
`docs/unsafe_catalog.md` 1.3.
