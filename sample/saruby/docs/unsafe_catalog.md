# `unsafe` in saruby: where it is necessary, what removing it cost, what removing it found

2026-11-22. The state described is master at **`f88d3e6`** ("saruby: todo.md (unsafe removal is
weighed against performance)"); every count below is of that tree unless a commit is named, except
the totals of 1.1 and 1.2, which are updated to master `d983d25` plus the zero-cost removals of
section 4.4 (371 -> 342). Raw
data, scripts and the experiment patches: `~/ruby/src/trials/2026-11-22-saruby-unsafe-catalog/`
(called `T` below). The other trials cited are siblings of it in `~/ruby/src/trials/`.

Sources: [safe_heap.md](safe_heap.md) (waves 1 to 4, the method layer, H1 to H5),
[safe_stack.md](safe_stack.md), [design.md](design.md), `../../../docs/gc.md`,
`tools/heap_audit.rb`, the trials named in each row, the ledger `~/ruby/src/ideas/`, and
`git log` of this repository.

Policy this document is measured against (docs/todo.md): unsafe removal is weighed against
performance ("性能と見合い"); method implementations are unsafe-free (every file of
`src/builtins/` except `sys.rs` is `#![forbid(unsafe_code)]`, done in the method-layer work).

How to read the numbers:

* **Measured** means a run is recorded in the cited trial; **assumed** or **argued** means it is
  not. Each table says which.
* Instruction counts (`instructions:u`) repeat to about 0.1 % and are the primary local metric.
  Cycle counts on the development machine (shared, load average 3 to 40 in every session cited)
  carry their spread; where the spread is as large as the difference, the text says that cycles
  cannot decide. sp4 (dedicated, performance governor) wall times are the only times quoted as
  times.
* There are no averages over benchmarks anywhere in this document: every number is one benchmark.

## Contents

1. [Inventory of the remaining `unsafe`](#1-inventory-of-the-remaining-unsafe)
2. [History: what was removed, how, at what cost, what it found](#2-history)
3. [The checked value stack against the raw one on sp4](#3-the-checked-value-stack-against-the-raw-one-on-sp4)
4. [Gaps measured for this document](#4-gaps-measured-for-this-document)
5. [What remains unmeasured](#5-what-remains-unmeasured)
6. [Findings for a paper](#6-findings-for-a-paper)
7. [Design note: the `&'static mut` accessors](#7-design-note-the-static-mut-accessors)
8. [Generated code](#8-generated-code)

## 1. Inventory of the remaining `unsafe`

### 1.1 Totals

Counted as `tools/heap_audit.rb` counts (line comments stripped; one per token, so `unsafe fn`,
`unsafe impl`, `#[unsafe(naked)]` and an `unsafe {}` block each count once).
`ruby tools/unsafe_catalog.rb` (added with this document) prints the per-category table of 1.2 and,
with `--list`, the category of every occurrence.

| where | `unsafe` | note |
|---|---:|---|
| `sample/saruby/src/**/*.rs` + `node.def`, hand-written | **342** | `heap_audit.rb` total after section 4.4 (master `d983d25`: 371; `f88d3e6`: 365) |
| of which compiled only with `FEATURES=raw_stack` | 5 | `src/stack_raw.rs`; the default build uses the checked `src/stack.rs` (0) |
| generated `src/node_*.rs` (from node.def by `lib/astrustgen.rb`) | 1152 | **93 after section 8** (`15ad997`: 1165); before it, after 4.4: node_specialize 402, node_hash 239, node_head 170, node_replace 122, node_dispatch 104, node_dump 104, node_eval 11; `prism_ast.rs` 0 (`f88d3e6`: 1131) |
| generated per bake: the code store's SD crates | 259 for `fib.rb` | `all.rs` items 100 (83 files), `patch.rs` items 159 (119 files); grows with the program |
| `runtime/` (ASTrust runtime shared by the samples) | 49 | `astrust.rs` 5 (dlopen / dlsym), `astrust_loader.rs` 10 (memfd / mmap), `gc/` 34 |
| `lib/astrustgen.rb` (templates of the generated code) | 32 | |
| `sample/astrogre` (the regexp engine crate linked into saruby) | 265 lines | `cps.rs` 20, `main.rs` 1, the rest generated `node_*.rs` (lines containing `unsafe`, `grep -c`); not catalogued further here |

The number of `unsafe` tokens understates the trust surface in one specific way: **safe functions
with unsound signatures** are still in use outside the converted modules. At `f88d3e6`, ten
functions return an unbounded `&'static mut` from safe code (`heap_audit.rb` column
`fn->&'static mut`: object.rs 7 — `obj`, `rhash`, `rclass`, `rproc`, `rmethod`, `rbig` and the
macro-made `rary` family —, vm.rs 2 — `vm()`, `Ctx::frame()` —, gc.rs 1 — `heap()`; 9 after
section 7 bounded `Ctx::frame()`; 8 after the vm / heap split of section 7: `vm()` returns `&'static Vm`,
object.rs keeps 7, and gc.rs's `heap()` is private to gc.rs; 1 after the object accessors of section 7:
only gc.rs's private `heap()`), and
`heap_audit.rb` counts **449 call sites** of the old accessors (rt.rs 189, vm.rs 169, node.def 25,
object.rs 25, legacy.rs 14, str_legacy.rs 11, lib.rs 11, gc.rs 2, aot.rs 1, compile.rs 1,
inline_builtin.rs 1; 0 object-accessor sites after section 7's object accessors, whose
`heap_audit.rb` sum of 149 is the `vm` column alone, i.e. uses of the shared `&'static Vm`). Each of
those was an aliasing or lifetime assumption that no `unsafe` keyword marks. One reduction of the
count was of exactly this kind: the ASTro-principles work (378 -> 365) replaced
`unsafe { (*c.frame).line = line; }` in node.def by `c.frame().pos = line;`, i.e. it moved the
dereference behind the safe-signature `Ctx::frame() -> &'static mut Frame`
(`2026-11-21-saruby-astro-principles/README.md`, section "unsafe"). The count of 365 is therefore
an upper bound on *marked* trust, not on trust.

### 1.2 By category

`ruby tools/unsafe_catalog.rb` at `f88d3e6` and after section 4.4 (master `d983d25` + the zero-cost
removals; the classification inside vm.rs / rt.rs / node.def and inside the heap files is by the text of
each site, a heuristic; `--list` shows every decision). Between the two columns lie a day of master
(the blop layer added 6 to sys.rs, the rest was neutral: 365 -> 371 at `d983d25`) and section 4.4 (-29).

| category | `f88d3e6` | after 4.4 | files (after 4.4) |
|---|---:|---:|---|
| OS interface: libc wrappers | 67 | 73 | builtins/sys.rs |
| OS interface: global allocator | 13 | 13 | kalloc.rs |
| fiber: stacks and context switch | 23 | 23 | fiber_os.rs 22, fiber.rs 1 |
| code loading: code store, patch binding | 11 | 6 | aot.rs |
| parser FFI (prism) | 1 | 1 | prism.rs |
| node building: raw node reads | 6 (+1 in text emitted into generated code) | 6 | compile.rs |
| GC & heap base | 70 | 61 | heap.rs 32, gc.rs 14, value.rs 7, object.rs 7, lib.rs 1 |
| call path & frames | 158 | 144 | vm.rs 76, rt.rs 57, node.def 11 |
| value stack, raw backend | 5 | 5 | stack_raw.rs (not in the default build) |
| String buffer (SBuf) | 10 | 10 | sbuf.rs |
| **total** | **365** | **342** | |

Sub-categories of the two large groups (`f88d3e6` -> after 4.4; one site, `Ctx::frame()` at vm.rs:409,
moved from "static mut global" to "frame chain" when the rule stopped matching the text `'static mut`):

| GC & heap base | `unsafe` | | call path & frames | `unsafe` |
|---|---:|---|---|---:|
| Value from a raw word (`from_raw`, `from_ptr`) | 15 -> 11 | | frame chain (`c.frame`, `prev`, `FrameFull`) | 39 -> 41 |
| entry / iseq / block / cref reads (leaked, H4) | 12 -> 10 | | method entry (leaked `Box`) | 36 -> 36 |
| static mut global (`VM`, `HEAP`, ...) | 8 -> 5 | | block pointer (H4) | 17 -> 17 |
| GC binding (gc.rs: `GcLang`, finalize, roots) | 8 -> 8 | | iseq (never freed) | 16 -> 16 |
| payload accessor (`body`, `body_mut`, `*_mut_and`, kind tests) | 8 -> 8 | | cref chain (leaked) | 11 -> 11 |
| legacy accessors (object.rs) | 7 -> 7 | | static mut global (`vm()`) | 10 -> 1 |
| object header place access | 5 -> 5 | | cache word reinterpreted (`as *const NewInfo`, `cc.poly`, `cc.val`) | 10 -> 8 |
| frame chain (H5) | 5 -> 5 | | node pointer | 5 -> 5 |
| context token (`Token::mint`) | 2 -> 2 | | Value from a raw word (literal caches) | 5 -> 0 |
| | | | GC root scan | 4 -> 4 |
| | | | unchecked kind assumption (`unreachable_unchecked`) | 3 -> 3 |
| | | | line position (`Pos`, patch holes) | 2 -> 2 |

### 1.3 Per site group: operation, why it is not safe, alternative, cost

"Cost" is the cost of the safe (or checked) alternative. **M** = measured (trial cited),
**A** = argued or assumed, not measured, **—** = no safe alternative in Rust.

#### OS interface

| file:line | operation | why not safe Rust | safe alternative | cost of it |
|---|---|---|---|---|
| builtins/sys.rs (67 at f88d3e6, 73 with the blop layer, 102 with socket, 109 with etc / syslog; one per wrapper) | etc (3, builtins/etc.rs): `sysconf`, `confstr`, `fpathconf`; syslog (4, builtins/syslog.rs): `openlog` (the wrapper owns the ident `CString` until `closelog`, since openlog keeps the pointer), `syslog` (a fixed `"%s"` format), `closelog`, `setlogmask`; socket (29, builtins/socket.rs): `socket`, `socketpair`, `bind`, `connect`, `listen`, `accept4`, `getsockname`, `getpeername`, `setsockopt`, `getsockopt`, `shutdown`, `sendto`, `recvfrom`, `sendmsg`, `recvmsg`, `nodename` (uname; all 16 through the already imported `syscall(2)`, since each new libc symbol costs ld.so time at every start), `getaddrinfo` (5: call, list walk, `ai_addr` copy, `ai_canonname`, `freeaddrinfo`), `getnameinfo`, `gai_strerror`, `getifaddrs` (6: call, family read, sockaddr copy, list walk, `ifa_name`, `freeifaddrs`); `read`, `write`, `pread`, `pwrite`, `open`, `close`, `dup`, `dup2`, `pipe2`, `poll`, `ppoll`, `epoll_create1`, `epoll_ctl`, `epoll_pwait2`, `eventfd`, `pidfd_open` (raw `syscall(434)`: glibc < 2.36 has no wrapper; the six are the blop layer's, io_design.md), `fcntl`, `flock`, `lseek`, `fsync`, `ftruncate`, `truncate`, `chmod`, `fchmod`, `chown`, `fchown`, `lchown`, `chdir`, `mkfifo`, `utimensat_cwd`, `umask`, `isatty`, `read_dir_raw` (3), `fd_metadata`, `waitpid`, `kill`, `fork`, `execvp`, `_exit`, `setsid`, `setpgid`, `getpgid`, `getsid`, `get*id` (6), `getrlimit`, `setrlimit`, `times`, `clock_gettime`, `clock_getres`, `nanosleep`, `set_itimer_real`, `signal`, `block_all_signals`, `setenv`, `unsetenv`, `clearenv`, `errno`, `set_errno`, `errmsg`, `local_tm` (2), libm `erf`, `erfc`, `cbrt`, `tgamma`, `lgamma_r` | FFI: a foreign function is `unsafe` to call. Each wrapper takes Rust types (slices, `&CStr`, `&mut`), so the pointer obligations are discharged by construction; one `SAFETY` line each (wave 4). Not dischargeable by a wrapper: `fork` (the child must exec at once or run alone), `setenv` / `unsetenv` / `clearenv` (race with `getenv` on other threads), `signal` (the handler's contract), `_exit`. | `std` covers part (`std::fs`, `std::process::Command`, `std::env::set_var`, which is itself `unsafe` since Rust 2024 for the same race); the rest needs a crate (`nix`, `rustix`) whose `unsafe` is the same FFI one level down. | A: moving the FFI into a crate changes where the `unsafe` is, not whether it exists. Not measured. |
| kalloc.rs:90–127 (`unsafe impl GlobalAlloc` + 4 `unsafe fn`), 70, 74, 88 (3 internal `unsafe fn`) | size-class allocator for blocks <= 1024 B on the interpreter thread | `GlobalAlloc` is an `unsafe trait`: an allocator hands out raw memory by definition. | the system allocator (`SARUBY_NO_KALLOC=1` switches it off at run time) | M: introduced with str 9601M -> 5517M, gcchurn 5014M -> 2228M, strfmt 8723M -> 4995M AOT instructions (commit f5249f4). At `f88d3e6` with `SARUBY_NO_KALLOC=1`: nesteddata +98.6 % / +141.4 %, rpartition_10 +66.7 % / +80.2 %, gen_gc +31.2 % / +49.7 %, strcmp +22.0 % / +30.6 % instructions (interp / AOT), ±1.5 % on str, strfmt, gcchurn, hash (section 4.2). |
| kalloc.rs:28, 35 | thread pointer by `asm!` (`fs:[0]` on x86-64, `tpidr_el0` on aarch64) | inline asm is `unsafe` | `std::thread::current().id()` or a `thread_local!` | A: one TLS access per allocation instead of one register read; not measured |
| kalloc.rs:61, 66 | `mmap` of the 64 GiB arena reservation; write of the arena statics | FFI; `static mut` | `std::alloc::System` for the arena (no reservation) | not measured |
| kalloc.rs:52 | `mallopt(M_TOP_PAD, 64 MiB)` | FFI | glibc tunables in the environment (`GLIBC_TUNABLES`) | M (`2026-11-21-saruby-sp4-regressions`): without it, `ary` on sp4 loses ~15K page faults per run to glibc heap trimming (67 ms vs 109 ms task-clock at 1cdddb3); the tunable works but must be set by whoever starts the process. Added on purpose (commit f495c0d). |

#### Fiber: stacks and context switch (`fiber_os.rs` 22, `fiber.rs` 1)

| file:line | operation | why not safe Rust | safe alternative | cost |
|---|---|---|---|---|
| fiber_os.rs:120–135 (`#[unsafe(naked)]` ×2, `unsafe extern "C" fn` ×2) | `switch_stack`, `fiber_start` in naked asm | switching the machine stack is outside Rust's abstract machine; naked functions and their asm are `unsafe` by definition | (1) one OS thread per fiber with a handoff (std only); (2) stackless coroutines (`async`), which would need the evaluator in continuation-passing form | A: (1) a thread handoff costs a futex wake + sleep per switch, against 403 cycles per resume+yield round trip measured for the asm switch (`2026-11-21-saruby-fiber`, `results/cycles_switch.txt`); not measured with threads. (2) not considered feasible for a tree-walking evaluator with native recursion. |
| fiber_os.rs:154, 156 (`unsafe fn switch` + body); fiber.rs:161 (the one call) | the switch itself, value and flag passed in registers | as above | — | — |
| fiber_os.rs:42, 63, 94, 97 | native stack: `mmap` (NORESERVE), `mprotect` guard page, `madvise(DONTNEED)` on release, `munmap` | FFI; raw OS memory | `Vec<u8>` as stack memory (no guard page, no lazy backing) | A: the previous design allocated and wrote the stacks eagerly: 500 dropped fibers held 1.36 GB RSS, vm_fiber_* were killed at 4 GB (`2026-11-21-saruby-fiber`, table "Results"). That was also a design without GC of fibers, so the number is not the cost of the `unsafe` alone. |
| fiber_os.rs:78 | write of the initial frame onto a fresh stack | raw pointer into memory Rust does not own as an object | — | — |
| fiber_os.rs:109 | the suspended stack's words for the GC (`stack::words`, clamped to the stack) | reading a range of another stack as `&[usize]` | — | — |
| fiber_os.rs:142, 175, 182, 189 | trampoline (`&mut *ctx`, `Value::from_raw`), `Token::mint` for the fiber's context, `FiberCtx` get / drop (`Box::from_raw`) | the context lives in a `Box` owned by the fiber and reached from the new stack by a raw pointer (self-reference through another stack) | — | — |
| fiber_os.rs:214, 232, 238, 241, 247, 258 | value stacks: `memfd_create` of a 4 MiB file of nil words, private `mmap`s of it, `from_raw_parts` -> `&'static [Slot]`, `madvise` on release | a slice over mapped memory | eager `vec![NIL; n]` (the previous design) | M: startup `p 1` 29M -> 16M cycles once the 64 MB main stack was no longer written at boot (`2026-11-21-saruby-fiber`); sp4 startup 0.023 s -> 0.008 s across 6cdf354 / de58efd (`2026-11-20-astrust-vs-astro/README.md`, partly other work). |

fiber.rs went from 21 to 1 `unsafe` in that trial (`deny(unsafe_code)` on the file; legacy accessors
17 -> 0, `static mut` 6 -> 0); the crate total went 378 -> 380 because fiber_os.rs (new) holds 22.

#### Code loading

| file:line | operation | why not safe Rust | safe alternative | cost |
|---|---|---|---|---|
| runtime/astrust.rs:168–191 (5) | `dlopen` the baked `all.so`, `dlerror`, `dlsym` the build id and each `SD_<hash>` | loading native code: what the symbol points to is trusted by construction (same rustc, same crate, build id checked) | no AOT (`--plain`) | M: the interpreter is the alternative; on sp4 at de58efd, fib 0.315 s interp vs 0.091 s AOT, tak 0.662 vs 0.211 (`results_sp4_all_de58efd.tsv`). |
| runtime/astrust_loader.rs:207–310 (10) | patch binding: `memfd_create`, `ftruncate`, `mmap` ×3 (RW view, RX view placed near libsaruby, fallback), `munmap`, the RW window as `&mut [u8]` (one writer process: a forked child binds into a memfd of its own, see 2.4 "found later"), `dlsym` of externals | writing and then executing code; the ELF reader and relocator themselves are safe Rust over byte slices | the shared SD without per-body copies (`SARUBY_PATCH=0`) | M (`2026-11-21-astrust-patch-binding`): optcarrot 84.74M vs 85.45M instructions per frame with / without (−0.83 %), cycles undecidable locally; tak −6.3 %, ackermann +3.0 % instructions; sp4 ackermann +7 % with patch binding (`2026-11-21-saruby-sp4-regressions`). |
| aot.rs:23, 30, 40, 69, 110 (5; **removed in 4.4**) | `static mut STORE` (the loaded code store) | `static mut` | done: a `thread_local!` `RefCell<Option<&'static mut TieredStore>>` (leaked as before, no TLS destructor, so the atexit stats hook still sees it; a nested borrow panics) | M: ±0.0 % instructions on all 30 cells of 4.4. |
| aot.rs:43, 84 | `transmute` of a `dlsym` address to `DispatchFn` | a data pointer to a function pointer | — (inherent to dlsym) | — |
| aot.rs:45, 82, 84 | write of the dispatcher into a node head | the node is a raw `NodePtr` (see "node pointers" below) | — | — |
| aot.rs:21, 53 | `atexit` (stats); the hole value read at `root + off` | FFI; raw read of a node field by offset | — | — |
| prism.rs:78 | the prism C parser: buffer new / serialize / free | FFI | a Rust parser (none for Ruby 4.0 at prism's level) | — |
| compile.rs:448–454, 970, 2160–2161 (6) | reads of node fields through `NodePtr` while building the tree | node pointers (below) | — | — |

#### GC & heap base (heap.rs 34, value.rs 11, gc.rs 16, object.rs 7, lib.rs 2)

| site | operation | why not safe | safe alternative | cost |
|---|---|---|---|---|
| heap.rs:65, 67 (`body`, `body_mut`), 109, 148, 204, 211 (`*_mut_and`), 51 (`kind_test!`), 185 (`float_val`) | the payload of a GC object as a reference that borrows the context (`c.ary(v) -> &[Value]`) | the object graph is owned by the collector, not by any Rust value: a `Value` is a word, its object is reached by address. The borrow of `Ctx` stands for "the heap"; the deref is the one place where that correspondence is asserted (H1 liveness, H2 one context at a time). | (b) `RefCell`-like borrow flag per object | M (`2026-11-21-koruby-safe-heap`, safe_heap.md "Alternatives"): ary AOT +96 % cycles / +13 % instructions, sieve +85 % cycles, fannkuch +66 %, sort +13 %; interpreter +0 to +7 %. Rejected. |
| heap.rs:35–43 | header place reads / writes (`klass`, `flags`) without a reference | the same address; place access avoids aliasing any payload borrow (H3) | `Cell` fields in `RObj` | A: same machine code (`Cell` is a place access); would make the header safe given a safe `&RObj`, which is what is missing. Not measured. |
| heap.rs:125, 128, 366; gc.rs:36, 40, 51 | `static mut VM`, `HEAP` behind `c.vm()`, `gc::heap()` (`STACK_BASE` and `STRESS` became `AtomicUsize` in 4.4, -2) | `static mut`; `vm()` has hundreds of context-free callers in rt.rs / vm.rs / node.def (the 449 legacy call sites) | `Vm` owned by `Ctx` (the stated end state, safe_heap.md "vm() and the method identities") | the check part measured in section 4 (`vmchk`); the ownership change not measured. |
| heap.rs:218–309 (12 + 5 frame) | method entries, crefs, iseqs, blocks and frames read through raw pointers, borrowing `c` | entries and crefs are leaked `Box`es, iseqs are never freed: never dangling, but stored as raw pointers because some are written in place (`me_set_visi`, `cref_set_refinements`) and because frames live on the native stack (H5) | `&'static` with `Cell` fields for the mutable parts (entries, crefs, iseqs); frames: no safe form (below) | A (entries): zero in principle; not built. |
| heap.rs:21; lib.rs:197; fiber_os.rs:175 | `Token::mint` (2 + 1) | the only way to make a `Ctx`; its contract (H2) is "contexts run one at a time" | — (an `unsafe` marker by design: it is what makes the rest checkable) | — |
| value.rs:18–21, 28, 30, 71, 76, 84, 86 (11) | `Value::from_raw` (NonZero unchecked), `from_ptr` | `Value` is `NonZeroU64`; `from_raw` is `unsafe` so that safe code cannot forge a heap `Value` (H1) | `NonZeroU64::new(..).unwrap()` / `.map(Value)` where the word is provably non-zero and not a heap address (fixnum, symbol, flonum, constants) | M: section 4 (`val`). In 4.4 only the four constants were taken (const fn, compile time: identical instructions); `fix` / `sym` / `flonum` stay unchecked because their safe forms move other functions' code (hash +1.4 %, nesteddata +7 % instructions, 4.4). |
| gc.rs:62, 71, 96, 102, 133–157, 183–186 | `from_ptr` of new objects, `unsafe impl GcLang`, `drop_body` / `finalize`, the conservative machine-stack scan | the collector frees memory and scans raw stack words | — | — |
| object.rs:119–143 (7) | the legacy accessors: `obj(v) -> &'static mut RObj`, `rhash`, `rproc`, `rmethod`, `rbig`, `rfloat_heap` with `unreachable_unchecked` on a wrong kind | unsound safe API kept for the 449 unconverted call sites | the context-borrowing accessors (done for every builtin) | M for the conversion: see section 2; the kind check is 2 instructions per access (pilot). |
| lib.rs:238 (**removed in 4.4**) | `rt::BOOTED = true` | `static mut` | done: `AtomicBool` | M: section 4 (`atomic`), 4.4. |

#### Call path and frames (vm.rs 80, rt.rs 62, node.def 16)

| group (count) | typical site | why not safe | safe alternative | cost |
|---|---|---|---|---|
| frame chain (39) | `(*c.frame).cref()`, `(*c.frame).blk`, `(*self.prev).cref()`, `&*(self as *const Frame as *const FrameFull)` (vm.rs:277), `(*g).prev` walks in rt.rs:500–519 | `Frame` records live on the native stack, one per Rust activation of a call, linked by raw `prev` pointers; `Ctx` holds the innermost. A reference with a lifetime cannot be stored in `Ctx` while the frame's owner (a Rust stack frame below) also uses it: the chain is self-referential through the machine stack. Lite frames are a prefix of `FrameFull`, distinguished by a flag. | frames in a `Vec<Frame>` owned by `Ctx`, linked by index (the value stack's `emu_idx` shape) | A: not built. The value stack's measurement of the same shape is the closest number: `emu_idx` AOT +16 to +89 % instructions (safe_stack.md, step 1), because the base and length are reloaded after every call. |
| method entry, leaked `Box` (36) | `&*me`, `(*me).kind`, `(*me).visi`, `(*me).owner` in `invoke*`, lookup, `respond_to`, `defined?` | `MethodEntry` is `Box::leak`ed and never freed, but kept as `*const` because `visi` is written in place (`Module#private`) | `&'static MethodEntry` with `visi: Cell<u8>` | A: no instruction change expected (a `Cell` read is a load). Not built. |
| block pointer, H4 (17) | `&*blk`, `(*blk).tag`, `(*blk).kind` | a `Block` lives in its creator's Rust frame or in a Proc; the callee receives `*const Block` and assumes it alive for the call (H4) | lifetimes on every builtin signature (`&'a Block`) | A: not attempted; it changes the `CFunc` type of every builtin. |
| iseq, never freed (16) | `&*is`, `(*is).bentry` | iseqs are leaked and immutable after build except `Cell` fields | `&'static Iseq` | A: zero; `c.iseq_ref` already returns `&'static Iseq` (heap.rs:271). |
| cref chain, leaked (11) | `(*cr).klass`, `(*cr).prev` loops (vm.rs:727, 917; rt.rs:350, 675) | leaked, `refinements` written in place | `&'static Cref` with a `Cell` | A: zero. |
| static mut global (10; 1 after 4.4) | `vm()` (vm.rs:470), `REFINEMENTS_ON` (vm.rs:1589, 2253; rt.rs:180, 746) and `BOOTED` (rt.rs:67, 107, 125) became `AtomicBool` in 4.4 | `static mut` | atomics (done); `Vm` owned by `Ctx` | M: section 4 (`atomic`, `vmchk`). Precedent: REDEF as `AtomicU32`, identical instruction counts (`2026-11-21-saruby-loops-arith`, step 0007, commit c9c69a5). |
| cache word reinterpreted (10) | `transmute::<usize, CFunc>(cc.cfunc)` (vm.rs:1350, 1465), `&*(cc.fast as *const NewInfo)`, `&mut *(cc.poly as *mut [CallCache; POLY_WAYS])`, `yc.seen as *const Iseq` | call caches are plain words so that baked code can test them with one compare and the generator can hash them | typed fields: `Option<CFunc>` (niche, same size), `Option<&'static NewInfo>`, `Option<Box<[CallCache; N]>>` | M for `cfunc`: section 4 (`cfunc`); applied in 4.4 (-2; object AOT -0.31 % instructions, the rest ±0.0 %). |
| node pointer (5) | `(*recv).head.kind`, dispatcher call `((*body).head.dispatcher)(c, body, fp)` | see "Generated code": nodes are a raw-pointer graph | — | — |
| Value from a raw word (5; 0 after 4.4) | literal caches `Value::from_raw(lc.v)` (node.def:70, 101, 108, 115, 191) | the cache is a `u64` | done: `LitCache { v: Option<Value> }` (niche) | M: section 4 (`val`), 4.4: ±0.0 %. The same change for the constant cache (`ConstCache.val`, node.def:311) costs one test per constant read in the interpreter (hash +0.28 %, str +0.28 %, gcchurn +0.20 %, object +0.12 %; 4.4) and was not applied. |
| GC root scan (4) | vm.rs:2331–2383: `visit_ctx(&mut *vm.main_ctx)`, class fields walked as words, live stack as `&[usize]` | the collector writes slots through raw addresses | — | — |
| unchecked kind (3) | rt.rs:268, 274, 278 `Body::Env` else `unreachable_unchecked` | legacy shape | `unreachable!()` / the checked accessor | A: 1 compare-and-branch per env access in the interpreter; not measured. Precedent: the kind check of the safe heap accessors, 2 instructions per access, 0 to +2.1 % instructions (pilot). |
| line position (2) | vm.rs:230 `Pos::bound` (an `unsafe fn`), 235 the deref of an untagged `Pos` | `Pos` is a tagged word: an address of a line (interpreter) or the line itself (bound by the patch loader) | an enum | A: the 48-byte frame variant that stored the line itself was measured and rejected: +50–60M instructions on method_call / send (`2026-11-21-astrust-patch-binding`, "Frame layout"). |

**stack_raw.rs** (5, only with `FEATURES=raw_stack`): the unchecked backend of the value stack.
The default build uses `stack.rs` (`#![forbid(unsafe_code)]`); its cost is section 3.

#### Generated code

Superseded by section 8 (the per-node templates hold no `unsafe` any more); the table is the state before it.

| where | count | what | why | alternative |
|---|---:|---|---|---|
| src/node_head.rs | 163 | `ALLOC_*` constructors and accessors: write fields of `NodeOf<T>` through `NodePtr` (`*mut NodeHead`), `(*n).head.line = line`, `node_laid`, `list_base` (`from_raw_parts` over child lists) | nodes are allocated once, laid out contiguously ("contiguous trees"), never freed, and rewritten in place: the dispatcher field is replaced at specialization and at the first dispatch of patch binding, and child slots are replaced by `node_replace`. Baked code addresses a node as `root + constant` and binds holes to node addresses, so the node's address is part of its identity. | an arena of nodes with `u32` indices and `Cell` fields: A, not built; it would change every `NodePtr` operand of every evaluator signature and the baked-code address arithmetic. |
| src/node_specialize.rs, node_hash.rs, node_dump.rs, node_replace.rs | 393, 235, 102, 120 | read node operands by kind to specialize, hash, dump, replace | as above | as above |
| src/node_dispatch.rs, node_eval.rs | 102, 16 | the dispatcher call through `(*n).head.dispatcher`; `match` over a child index with `unreachable_unchecked` | as above | the default arm as `unreachable!()`: A, not measured |
| code store SD crates (per bake; fib: 259) | | every `Disp::call` body is one `unsafe { .. }` (operand reads through `np`, `&mut (*np).u.cc`), `DispList::nth`'s `unreachable_unchecked`, `Value::from_raw(<literal>)` (`const { Value::imm(<literal>) }` since 4.4), and in `patch.rs` the hole items: `Pos::bound(&raw const ASTRUST_HOLE_line_N)` and the `asm!` store of a relocated immediate | the generator emits node reads as raw pointer reads; patch binding needs a relocation the compiler cannot express in safe Rust (`asm!` with a `sym` operand) | — for the holes; the rest as for node pointers |

#### String buffer (sbuf.rs 10)

| site | operation | why | safe alternative | cost |
|---|---|---|---|---|
| sbuf.rs:20–169 | `SBuf` = 3 words: 23 bytes inline (length in the last byte) or a Vec-compatible heap block (tag in the capacity's top bit); `from_raw_parts` / `Vec::from_raw_parts` over the buffer it alone owns, inline copies | a union of an inline array and a `Vec` in 24 bytes, with the discriminant in spare bits, cannot be written as a safe enum at that size | `boxvec` (`Inline \| Box<Vec>`), `vec15` (15 inline bytes), `rs29` (32-byte enum, 29 inline, encoding in the spare byte; branch `sbuf-safe-rs29`) | M (`2026-11-21-saruby-unsafe-method-layer`): rs29b str +4.6 % / +6.0 % instructions (interp / AOT), strscan +9.4 %, interpolation_same_heap +6.5 % (+49 % / +62 % cycles), faster where 24–29-byte strings stay inline (strfmt −12 % / −25 % cycles, binary_concat_embedded −27 % / −40 %). Not merged. At `f88d3e6` (section 4.2) the instructions are the same: str +4.3 % / +6.1 %, strscan +9.4 %, to_words_10 +7.8 / +7.9 %; the earlier cycle wins on binary_concat_embedded did not reproduce. |

## 2. History

### 2.1 Count of `unsafe` per commit

`T/scripts/unsafe_history.rb` (comments stripped as heap_audit does; the sample was `sample/koruby`
before the rename). `T/results/unsafe_history.tsv`.

| commit | date (UTC) | what | hand-written | of which node.def | generated node_*.rs | runtime/ | forbid files | `static mut` |
|---|---|---|---:|---:|---:|---:|---:|---:|
| 2ae4b98 | 11-20 14:21 | before the safety work | 651 | 50 | 787 | 5 | 0 | 52 |
| 63174bc | 11-20 16:30 | value stack as an API, two backends | 566 | 22 | 759 | 5 | 2 | 52 |
| 5453803 | 11-20 17:10 | checked stack the default (+ GC framework in runtime/) | 564 | 22 | 759 | 39 | 2 | 55 |
| 38126da | 11-20 18:55 | heap pilot (Array) merged | 570 | 22 | 759 | 39 | 4 | 48 |
| dc28c5a | 11-20 21:12 | wave 2 (hash, range, numeric, struct, enum) | 567 | 22 | 759 | 39 | 15 | 48 |
| 261e998 | 11-20 22:08 | String side merged | 561 | 24 | 768 | 39 | 22 | 41 |
| 5791694 | 11-20 22:16 | (base of wave 3; ivar / arith work merged) | 565 | 24 | 899 | 39 | 22 | 41 |
| e9ece96 | 11-21 00:23 | wave 3 merged | 498 | 27 | 905 | 39 | 34 | 36 |
| 52842b4 | 11-21 01:28 | wave 4 (sys.rs wrappers, `Global`) | 376 | 27 | 905 | 39 | 46 | 20 |
| 1b1b311 | 11-21 07:20 | fibers: fiber.rs 21 -> 1, fiber_os.rs 22 | 380 | 27 | 907 | 39 | 49 | 14 |
| 322b70f | 11-21 10:58 | patch binding + loader | 375 | 16 | 996 | 49 | 49 | 16 |
| c9c69a5 | 11-21 13:40 | REDEF an `AtomicU32` | 373 | 16 | 997 | 49 | 49 | 15 |
| 0059037 | 11-21 14:10 | method layer (binding, boot, enc_tables) | 366 | 16 | 996 | 49 | 52 | 16 |
| **f88d3e6** | 11-21 18:27 | this document's state | **365** | 16 | 1131 | 49 | 52 | 15 |
| (blop) | 11-22 | blop layer (docs/io_design.md): 6 sys.rs wrappers (`ppoll`, `epoll_create1`, `epoll_ctl`, `epoll_pwait2`, `eventfd`, `pidfd_open`); `builtins/blop.rs` is `forbid` | 371 | 16 | — | 49 | 55 | 15 |
| (socket) | 11-22 | socket.so natively (`builtins/socket.rs`, `forbid`): 29 sys.rs wrappers (16 socket syscalls + uname via `syscall(2)`, getaddrinfo 5, getnameinfo, gai_strerror, getifaddrs 6) | 400 | 16 | — | 49 | 57 | 15 |
| (stdlib-2) | 11-22 | `2026-11-22-saruby-stdlib-2`: digest, zlib, psych, bigdecimal, socket, StringIO port, syslog, etc, cgi/escape, coverage (all new files `forbid`); 7 sys.rs wrappers for etc (3) and syslog (4) | 407 | 16 | — | 49 | 57 | 15 |

(The table is by commit, not by date order of merge; merges carried other agents' work, so
neighbouring rows differ by more than the named change. The generated count grows with
node.def, not with the safety work.)

### 2.2 Removal waves

| wave (trial) | commits | `unsafe` before -> after (whole crate) | technique | cost, per benchmark (instructions unless stated) | bugs found |
|---|---|---|---|---|---|
| value stack (`2026-11-20-koruby-safe-stack`, safe_stack.md) | 63174bc, 5453803 | 651 -> 566 (node.def 50 -> 22, vm.rs 101 -> 68, rt.rs 81 -> 62); stack module `forbid` | `Cell<Value>` slots; frame window `&'static [Slot; 65536]` (one word, literal indices checked at compile time); position slices from `Ctx`; sp as free-slot count; raw backend kept as a feature | AOT fib +7.1 %, tak +17.1 %, collatz +7.5 %, nbody +2.5 %, method_call +0.0 %, loops +0.0 to +1.1 %; interp fib +6.4 %, tak +8.0 %, method_call +7.0 %. Alternatives measured: `emu_idx` +16 to +89 %, fat slice +8.4 to +16 %. sp4: section 3. | none: the checked backend turned no access of the test corpus into a panic (1699 t/hand files identical to the raw build); i.e. rule R1 held |
| heap pilot: Array (`2026-11-21-koruby-safe-heap`) | ac88a59, 02fd48b, a9bd97d, 38126da | 564 -> 570 (heap.rs +12: the `unsafe` moved into two functions) | context-borrowing accessors (`c.ary(v)` borrows `Ctx`; allocation takes `&mut Ctx`); `heap::no_legacy` ban module; method identities off `static mut`; `Token::mint` | AOT ary +2.1 %, array_access +1.4 %, mapreduce +1.4 %, cmpsort +0.2 %; cycles −3.9 to +1.6 % (inside spreads). `nokind` would save 0 to 2.1 %. RefCell alternative: ary +96 % cycles. | the RefCell emulation's first run panicked at a site that is correct under the borrow design (`Array#initialize`): 93 places in array_methods.rs that compile under a guard design and are latent panics or references across a GC point |
| wave 2: hash, range, struct, enum, numeric (`2026-11-21-saruby-safe-heap-hash-num`) | af4b634..dc28c5a | 570 -> 567 | `*_mut_and` + `Others`; slot-by-slot iteration with `iter_lev`; re-validate after `eql?`; Copy snapshots | the checks: nesteddata +1.3 % / +2.4 % (32 instructions per iteration). Restructuring forced by the borrow rule: hash −18 % / −36 %, methodchain −4.1 / −4.9 %, mapreduce −2.8 / −3.6 %. | 6 fixed (table 2.4) |
| String side (`2026-11-21-saruby-safe-heap-string`) | d43563c..1c403de | 570 -> 558 (encoding.rs 10 -> 0, strscan.rs 3 -> 0) | as above + `str_mut_and`, `ByteSnap` (stack copy), `#[inline(always)]` thin accessors | converted as it stood: strfmt +0.2 %, strcmp +2.2 %, strops +3.3 %, strscan +8.7 %; after tuning ≤ +0.2 % (strcmp). A store-forwarding stall: strscan −3 % instructions, **+30 % cycles** until the constructor was inlined. symbol table `Mutex`: +8 % (rejected). | 7 fixed (table 2.4) |
| wave 3: rational .. symbol (`2026-11-21-saruby-safe-heap-3`) | 45694a5..866baed | 565 -> 485 (heap.rs 19 -> 33) | frame-chain read accessors (H5), `me_ref` / `cref_ref` / `iseq_ref`, `with_iseq`, `alloc_with` closures; symbol table lock-free names + `AtomicU32` intern cache | within 0.1 % on 10 benchmarks; symbench −9 % / −15 % (the cache; without it the safe table costs +4 to +6 %); bignum AOT +5.2 to +7.9 % cycles on equal instructions with `intern` inlined (placement; taken out of line) | 7 fixed, incl. 3 SIGSEGV and a GC rooting bug (table 2.4) |
| wave 4: marshal .. pp (`2026-11-21-saruby-safe-heap-4`) | 580e813..52842b4 | 498 -> 376 (sys.rs 8 -> 67) | every libc call behind a safe wrapper in sys.rs; `heap::Global` (pinned slot index in an `AtomicUsize`); atomics for flags | io_write −9 % / −11 %, json −10 %, marshal −1.5 %; exception +0.1 / +0.3 % (inlining moved); strfmt AOT +2.2 to +3.9 % cycles on equal instructions, unresolved locally | 17 fixed, incl. 1 SIGSEGV, 1 UB (`from_utf8_unchecked`), 2 GC rooting (table 2.4) |
| fibers (`2026-11-21-saruby-fiber`) | 7e3e866, 1b1b311 | 378 -> 380 (fiber.rs 21 -> 1, fiber_os.rs 0 -> 22) | OS/switch module with one operation per `unsafe`; fibers traced through their Fiber object | switch: +10 % instructions per resume+yield (854 -> 943) but **−15 % cycles** (477 -> 403); fib, gcchurn, iterators, binary_trees, hash, ackermann, gen_gc within ±0.1 % | a double resume (`a` resumes `b` resumes `a`) corrupted the stack (panic "entered unreachable code"); no guard page on fiber stacks; 500 dropped fibers 1.36 GB RSS |
| REDEF (`2026-11-21-saruby-loops-arith`, step 0007) | c9c69a5 | −2 (and the one step 0002 had added) | `static mut u32` -> `AtomicU32` (Relaxed) | identical instruction counts (bitops / while / intdiv equal to r9) | — |
| blop layer (`2026-11-22-saruby-blop`, io_design.md) | this branch | 365 -> 371 (sys.rs 67 -> 73) | the new kernel interfaces (ppoll, epoll, eventfd, pidfd) as one-line safe wrappers; the slab / engines / scheduler are `forbid(unsafe_code)`: in-flight ops are owned by the slab and named by an `Id`, so no pointer crosses a fiber switch | measured as behaviour, not instructions: idle wake-ups 938 -> 2-4 per second, pipe round trips through `cat` 1.1k/s -> 56k/s (trial README) | — |
| etc / syslog (`2026-11-22-saruby-stdlib-2`) | this branch | 400 -> 407 (sys.rs 102 -> 109) | `sysconf` / `confstr` / `fpathconf` take integers and a slice; `openlog` takes the ident by value and keeps it in a static until `closelog` (libc keeps the pointer); `syslog` passes the message as the argument of a fixed `"%s"` | not on any hot path; startup unchanged (`Etc` / `Syslog` load on require) | — |
| socket (`2026-11-22-saruby-stdlib-2`, worker socket) | this branch | 371 -> 400 (sys.rs 73 -> 102) | one-line wrappers taking slices / `&mut` locals; a sockaddr is a byte slice in the kernel layout, built and parsed in safe Rust (`socket.rs`); getaddrinfo / getnameinfo copy their results out before freeing; the resolver runs on a `std::thread` while other Ruby threads exist (a `Send + 'static` closure, woken through an eventfd) | startup: +14k instructions with codegen-units=1 (ld.so: 6 new libc symbols, relocations) | — |
| method layer (`2026-11-21-saruby-unsafe-method-layer`) | 85a975f, 61270f2, 0059037 | 375 -> 366 | binding.rs on the accessors; boot as a struct literal (`blank_vm` with `MaybeUninit` removed); `heap::boot_vm` once-guarded | none measured on the converted paths; SBuf safe forms measured and not merged (1.3) | `blank_vm` left any field added after `main_obj` uninitialized without a diagnostic (now E0063) |

### 2.3 `unsafe` added (for speed or for a feature)

| change (trial) | `unsafe` added | what it bought (measured) |
|---|---|---|
| call path round 1 (`2026-11-21-saruby-call-path`) | 4: `&*c.call_blk`, `&*fr.h.iseq()` in `node_block_entry`, dispatcher call in `srb_block_redo`, `self_blk` read in `Frame::iseq()` | block −7.1 %, iterators −6.9 %, mapreduce −5.2 % instructions |
| call path round 2 (`2026-11-21-saruby-call-path-2`) | 12 (5 compiled-in yield, 1 shared-frame line store, 5 bake-time, 1 counted as moved); the README first said none were added and was corrected | u_yield 179 -> 148 instructions per call+yield, b_yield_each −18 % instructions / −26 % cycles; u_fcall0 57.0 -> 37.8; optcarrot 101.73M -> 99.00M per frame |
| keyword dense entries (`2026-11-21-saruby-kwargs-closures`) | 4 added (`kw_dense_fill` ×2, `call_proc_kw`, `finish_iseq`), 2 removed (`get_unchecked` in `env_get` / `env_set`: EnvVals indexes safely and folds) | kwargs −71 % cycles; u_lambda_call 306 -> 219 |
| patch binding (`2026-11-21-astrust-patch-binding`) | loader 8, aot.rs 7, vm.rs 2 (`Pos`), generated holes (asm store) | optcarrot −0.83 % instructions per frame; RSS +9 MB; startup +2.7M instructions |
| recursion (`2026-11-21-saruby-recursion`) | 2 at bake time in the generator's output; none at run time | ackermann −24 % cycles (one-level unroll) |
| `mallopt` (`2026-11-21-saruby-sp4-regressions`, commit f495c0d) | 1 | sp4 ary 109 -> 67 ms task-clock (page faults 41K -> 15K) |
| alloc sweep (`2026-11-21-saruby-alloc-sweep`) | `unsafe fn drop_body` (two statements moved out of an existing block) | — (not for speed) |

Proposed and **rejected** (each in the ledger `~/ruby/src/ideas/rejected/`):

| proposal | gain measured | why rejected |
|---|---|---|
| ivar read without the body-kind test (`unreachable_unchecked` after a class-cache hit; `saruby-ivar-kind-check-unsafe`) | optcarrot 107.88M -> 104.87M instructions per frame (−2.8 %) | new `unsafe` resting on "a class whose cache was filled has only plain-object instances"; policy |
| cfunc arguments passed zero-copy through one cast (safe_stack.md) | at most 2.4 % (mathfn AOT), within 0.25 % on eight of eleven | not worth an unsound cast |
| frame-id / REDEF / method serial per context (`2026-11-21-saruby-recursion`, call-path) | one load per op / call | every bump would write other contexts through raw pointers while `&mut Ctx` borrows of them are live |
| native stack check by guard page instead of the compare (`2026-11-21-saruby-recursion`) | not built | needs a signal handler |
| line store removed via `MaybeUninit` (`2026-11-21-saruby-call-path-3`) | not built | `unsafe` |

### 2.4 Bugs found by removing `unsafe`

Each line is a defect of the code *before* the conversion, found while its module was converted
and fixed in the commit named. Repro files are in the trial named. "now" = CRuby's output.

**Found by** says what first exposed the defect (one value per row):

* **a**: a run-time check of the safe API fired (the kind check of a context-borrowing accessor
  panicked).
* **b**: the borrow checker rejected the converted code at the defect's site (E0499 / E0502), and
  the fix is what made it compile.
* **c-read**: named in the code while it was being rewritten: a fix was written before any run showed
  the defect, or the converted build already behaved correctly the first time it ran the repro.
* **c-test**: a run showed it first: a smoke or callback test written during the conversion, the
  per-file run of the test corpus against CRuby, a `SARUBY_GC_STRESS=1` run, or a crash in a
  benchmark.
* **undetermined**: the records do not say (no row needed it).

How it was decided: for each row, the fix commit (named; `git show` contains the fix), and the
recorded session of the agent that did the conversion, read in order: which compile errors, panics and
test outputs came before the fix, and whether the build the repro first ran on was already fixed. The
records hold commands, edits and outputs but not the reasoning behind them, so c-read says that the fix
came before any run showed the defect, not why that code was looked at. The per-row record numbers are in `T/README.md`, section "bug attribution". Where a pre-fix shape was
cheap to rebuild, it was written with today's safe API and type-checked (2.4.1).

Correction: until 2026-11-22 this section attributed the rows by their kind ("unchecked kind" rows,
the SIGSEGVs, to the accessors' kind check; "stale reference" rows to the borrow rule) without
checking how each was found. The records contradict that: none of the five SIGSEGV rows was found by a
check firing, and one stale-reference row was found by the borrow checker. The column below replaces
it.

| # | wave | repro (one line) | before | kind | found by | evidence (fix commit: what exposed it) |
|---:|---|---|---|---|---|---|
| 1 | 2 | `h = (1..100).to_h { [_1, _1] }; n = 0; h.each { \|k, _\| h.delete(k); n += 1 }; p [n, h.size]` | `[59, 41]` (slots compacted under the iteration) | iterator invalidation | c-read | af4b634: hash.rs was written anew (no compile error in it); the repro list naming rows 1 to 4 was written before the first run, on which the new build already printed `[100, 0]` |
| 2 | 2 | `class K; def initialize(h) = @h = h; def hash = 7; def eql?(o) = (@h.clear if $go; false); end; h = {}; 12.times { h[K.new(h)] = 1 }; $go = true; p h[K.new(h)]` | panic index out of bounds (probe kept the mask across `eql?`) | stale index across a callback | c-read | af4b634: same list; the panic is Rust's slice bounds check in the base code (hash.rs:136), not a check the conversion added; the pre-fix shape compiles on the safe API (2.4.1) |
| 3 | 2 | `o = Object.new; def o.hash = raise("boom"); a = [o]; 2.times { begin; a.hash; p :no; rescue => e; p e.message; end }` | `"boom"`, then `:no` (recursion guard left set) | state not restored on raise | c-read | af4b634: same list |
| 4 | 2 | `h = {"a" => 1}; begin; h.each { h.compare_by_identity }; rescue => e; p e.message; end; p h["a"]` | wrong message, `nil` | flag set before a failing rehash | c-read | af4b634: same list |
| 5 | 2 | `h = {}; h.replace({}.compare_by_identity); p h.compare_by_identity?` | `false` | | c-read | af4b634: second repro list; the new build was correct on its first run |
| 6 | 2 | `S = Struct.new(:a); s = S.new(nil); s.a = s; p s.hash == s.hash` | process dies (unbounded recursion) | | c-test | dc28c5a: smoke test of the struct conversion; base and new build both died |
| 7 | String | `s = +"abc"; o = Object.new; o.define_singleton_method(:to_str) { s << "y" * 300; "c" }; s.chomp(o).size` (also `chomp!`, `split`) | 3 (bytes of a moved buffer; CRuby 303) | **use of a stale `&'static [u8]` after the buffer moved** | **b** | b92b160: the first `cargo check` after the mechanical pass over string_methods.rs gave E0502 at `let suf = expect_str(c, a[0])?` in `chomp_impl` (the receiver's bytes borrowed across `to_str`) and at `get_pat` in `split`; both were rewritten to read the receiver after the conversion. The callback test later showed base 3, new 303. Rebuilt: E0502 (2.4.1) |
| 8 | String | `io.read(o)` with `o.to_int` doing `io.string = "x"` | `"lo w"` (old string) | stale reference across `to_int` | c-read | f1ecd68: stringio.rs rewritten on a `Copy` snapshot, no compile error; the build the callback test first ran on already printed `"x"`. Rebuilt: E0499 with `&mut StrIO`, compiles with the snapshot (2.4.1) |
| 9 | String | `Class.new(StringIO) { def initialize; end }.new.read` | **SIGSEGV** (nil read as a String) | unchecked kind | c-read | f1ecd68: the rewritten stringio.rs guarded "a stream whose initialize never ran"; the repro was then run on base (status 139) |
| 10 | String | `s[o, 2] = "x"`, `s.byteslice(o, 5)`, `s.bytesplice(o, 5, "x")` with a `to_int` that clears `s` | panic (Rust index out of range) | length read before a conversion | c-test | 1d4f2cb: callback test (91 cases, one process each); base and the converted build both panicked in Rust's bounds check; the shape compiles (2.4.1) |
| 11 | String | `s.each_byte { s.clear }` | 150 iterations, zeros | iterating a buffer that is gone | c-test | 1d4f2cb: callback test; base and converted build both wrong; the shape compiles (2.4.1) |
| 12 | String | `split` / `scan` / `gsub` / `scrub` with a block that changes the receiver | no "string modified" | | c-test | 1d4f2cb: callback test and a split/scan/gsub variant file |
| 13 | String | `sub`, `partition` argument / receiver order | read at the wrong moment | | c-test | 1d4f2cb: callback test |
| 14 | 3 | `abort(1)` | **SIGSEGV** (a Fixnum read as a String) | unchecked kind | c-read | e3c62b2: a grep for accessor calls in the converted kernel.rs listed `c.str_bytes(m)` on an unchecked argument; `expect_str` was added before the repro ran (base: SIGSEGV) |
| 15 | 3 | `autoload(1, "x")` | **SIGSEGV** | unchecked kind | c-read | e3c62b2: the same grep listed the `autoload` lines; the argument checks were added in the same edit, before the repro ran |
| 16 | 3 | `autoload(:X, 1)` / `autoload(:X, "")` | accepted | missing argument check | c-read | e3c62b2: the same edit |
| 17 | 3 | `Module.new { refine(String) { import_methods 1 } }` | **SIGSEGV** | unchecked kind | c-read | 93d8b24: a grep for `c.class(..)` on arguments in object.rs; the check was added with no run in between |
| 18 | 3 | `class String; def upcase = :x; end; p :y.upcase` | panic `:x is not a heap object` | unchecked kind | **a** | 4985d89: the panic scan of 2949 output files found `saruby: :redef_up is not a heap object` at heap.rs:26 (`wrong_kind`, the kind check added with the heap pilot) in t/hand/sdi_inline_sends.rb on base 5791694 (`2026-11-21-saruby-safe-heap-3/results/pan.base/`); the commit message: "segfaulted before the safe-heap conversion" |
| 19 | 3 | `"ab" =~ /(?<x>a)/; p Regexp.last_match(:x)` | `nil` (name read with `as_fix`) | unchecked kind | c-read | f070c01: regexp_methods.rs read whole and rewritten; `as_fix` replaced before the repro ran |
| 20 | 3 | `SARUBY_GC_STRESS=1`: `p(/\A(?<x>.)(?<x>.)\k<x>\z/.match("abb").to_a)` | `["b", "b", "b"]` | **GC rooting: Strings held only in a Rust `Vec` during an allocation** (MatchData#to_a) | c-test | f070c01: GC-stress run of the regexp files (4 differ from base); the shape compiles (2.4.1) |
| 21 | 4 | `r, w = IO.pipe; w.close; r.read(5, 123)` | **SIGSEGV** (an Integer read as a String) | unchecked kind | c-read | a7c9c80: io.rs was written anew (no borrow error); the new build was correct on the first repro run (rows 21 to 26 in one file) |
| 22 | 4 | `r, w = IO.pipe; w.close; r.read(5, "x".freeze)` | nil | missing frozen check | c-read | a7c9c80: same run |
| 23 | 4 | `IO.read(path, -1.5)` | "negative length -9007199254740989" (a Float read as a Fixnum) | unchecked kind | c-read | a7c9c80: same run |
| 24 | 4 | a `to_int` that closes the stream during `r.read(o)` | nil via a stale reference | stale reference across a callback | c-read | a7c9c80: same run. Rebuilt in the pre-fix shape (`&mut RIo` across `num_to_long`): E0499 (2.4.1); the compiler never saw it because io.rs was not converted line by line |
| 25 | 4 | `r.read(-1)` on a closed stream | "closed stream" (CRuby: "negative length -1 given") | order of checks | c-read | a7c9c80: same run |
| 26 | 4 | `File.realpath` / `Dir.pwd` on a name with a latin-1 byte | bytes replaced by U+FFFD | **undefined behaviour: `from_utf8_unchecked` on non-UTF-8 bytes** | c-read | a7c9c80: same run; the `unsafe` in `path_arg` was itself what the conversion removed |
| 27 | 4 | `File.readlines(dir)`, `File.foreach(dir)` | `[]`, nil (CRuby: Errno::EISDIR) | error dropped | c-test | a7c9c80: per-file run of t/hand: `file_read_dir_eisdir.rb` changed, then checked against CRuby |
| 28 | 4 | `IO.readlines(path, chomp: true)` under `SARUBY_GC_STRESS=1` | TypeError (options Hash collected) | **GC rooting** | c-test | a7c9c80: GC-stress per-file run (`io_class_methods.rb`) |
| 29 | 4 | `JSON.load(src, proc)` | proc called on values only, result ignored | compatibility | c-test | 1103063: smoke test of the converted json.rs against CRuby |
| 30 | 4 | `JSON.load("  ")`, `JSON.load("", nil, allow_blank: false)` | nil (CRuby: JSON::ParserError) | compatibility | c-read | 1103063: found in json 3.1's `JSON.load` source while fixing row 29 |
| 31 | 4 | `marshal_dump` / `to_json` that adds a key to the Hash being dumped | dumped silently | iterator invalidation | c-read | edc8776, 1103063: Hash walks rewritten slot by slot inside `hash_iter_enter`; the new build was correct on the first repro run |
| 32 | 4 | `marshal_dump` that clears its Hash and runs `GC.start` | dumps a freed String (79 bytes) | **use after free (GC)** | c-read | edc8776: same run; the old code dumped from a `Vec` of pairs; the shape compiles (2.4.1) |
| 33 | 4 | `SystemCallError.new("x", 2)`, `SystemCallError.new(2)` | a SystemCallError (CRuby: Errno::ENOENT) | compatibility | c-test | 66320e3: smoke test of the converted exception.rs against CRuby |
| 34 | 4 | `SystemCallError.new("x", 9999).message` | "unknown error - x" | compatibility | c-test | 66320e3: same test |
| 35 | 4 | `SystemCallError.new("message").errno` | the module `Errno` (found as `Object::Errno`) | compatibility | c-test | 52842b4: per-file run of t/spec/exception_000.rb against CRuby |
| 36 | 4 | `SystemExit.new.message`, `SystemExit.new(3).message` | "exit" (CRuby: "SystemExit") | compatibility | c-test | 66320e3: smoke test |
| 37 | 4 | `RuntimeError.new("a\nb").inspect` | `#<RuntimeError: "a\nb">` | compatibility | c-test | 66320e3: smoke test |
| 38 | fiber | `a` resumes `b` resumes `a` | panic "entered unreachable code" (stack corrupted) | | c-test | 1b1b311: a test of Fiber's error cases run on the base build before the rewrite; the panic is an `unreachable!()` present since b3cc05f (2026-11-20), not a check of the conversion |
| 40 | object accessors | `$go = false; class K; def initialize(s) = @s = s; def hash; if $go; $go = false; @s.clear; GC.start; end; 1; end; end; src = {}; src[K.new(src)] = "a"; 40.times { src["k#{_1}"] = "v#{_1}" * 10 }; $go = true; p({**src}.size)` | **panic** `not a class: #<obj ..>` (a freed object's klass); under `SARUBY_GC_STRESS=1` 28 entries holding other Strings | **GC rooting / use after free: pairs copied to a Rust `Vec` across `hash_aset` (Ruby `#hash`)** | c-read | 9e7fed4: while `rhash` / `hash_pairs` were replaced in rt.rs / vm.rs (the replacement `c.hash_pairs(src)` compiles), the loop was recognised as pattern 14 of safe_heap.md; the fix (`hash::hash_copy_entries`, slot by slot) was written before the repro, which then panicked on base `1749107`. Same shape at 5 sites (`{**h}` / `f(**h)`, two ruby2_keywords splats, `**kwrest` and unknown-keyword collection, pattern matching's rest); only `{**h}` reproduced (`2026-11-22-saruby-static-mut/objacc/repro/`) |
| 39 | method layer | a `Vm` field added after `main_obj` | left uninitialized (`MaybeUninit` + `ptr::write`), no diagnostic | **latent UB** removed by construction | c-read | 0059037: boot rewritten as a struct literal (a missing field is now E0063); never observed |
| 41 | gc heap | `p [:a, :b].cycle(2**62).size` | debug build: panic (i64 overflow in `len * n`); release: wrapped (CRuby master 9223372036854775808) | integer overflow | c-test | e521210: the dev-build panic scan of the gc heap conversion (`cycle_nb_io_timeout_1020.rb`, trial README "gc heap"); fixed in the NO_CTX work |
| 42 | gc heap | `p({a: 1, b: 2}.cycle(2**62).size)` | 0 (`Enumerable.__int` made a Fixnum of a long past the Fixnum range) | integer overflow | c-test | e521210: the first smoke test of row 41's fix (both `cycle` sizes printed side by side) printed 0; the cause read in `e_int` |
| 43 | gc heap | `p [:a, :b].cycle(2**62).size` after row 41's fix | nil (`Enumerator#size` passed only Fixnum / Float sizes) | compatibility | c-test | e521210: the same smoke test printed nil; the cause read in `e_size` |
| 44 | inline caches | AOT: a call site re-entered by recursion while the method serial changes refills its cache; then `x.f` with an A runs B#f (`tools/aot/ic_reentry_spec_klass_1123.rb`) | `[:A, :A, :B, :B]` (CRuby `[:A, :A, :A, :B]`; `--plain` right) | **undefined behaviour used by LLVM: `&mut CallCache` (`noalias`) live across a call that re-enters the same node** | c-read | f507c19: named in 8.3 item 4 while the generated code's `unsafe` was removed; the repro was written after reading `call_spec_out`'s machine code (pre-call `cc.klass` reused after the call), failed on base, passed on base rebuilt with `-Zmutable-noalias=no` and on the Cell form (trial `2026-11-23-saruby-ic-borrow`) |

Rows 22, 25, 27, 29, 30, 33 to 37 are the "10 more compatibility differences" of earlier versions of
this table; row 16 was counted in wave 3's 7 but missing from the table.

Counts (rows of the table above; no row is undetermined; the "found later" row below is c-test and not counted):

| found by | rows | SIGSEGV | UB (incl. latent) | GC rooting / UAF | wave 2 | String | wave 3 | wave 4 | fiber | method layer | object accessors | gc heap | inline caches |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| a: run-time check of the safe API | 1 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 0 | 0 | 0 |
| b: borrow checker (compile time) | 1 | 0 | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| c-read: reading while rewriting | 24 | 5 | 3 | 2 | 5 | 2 | 5 | 9 | 0 | 1 | 1 | 0 | 1 |
| c-test: a test run during the work | 18 | 0 | 0 | 2 | 1 | 4 | 1 | 8 | 1 | 0 | 0 | 3 | 0 |
| **total** | **44** | 5 | 3 | 4 | 6 | 7 | 7 | 17 | 1 | 1 | 1 | 3 | 1 |

Borrow errors that were not defects. The first `cargo check` of string_methods.rs gave 21 E0502 and
16 E0499 at about 50 lines; those in `chomp_impl` and `split` are row 7, the others are not tied to any
row (for example an allocation inside the argument list of another call that takes the context,
`c.ary_from_vec(vec![c.str_new(..), ..])`). The other conversion records show 26 borrow-error messages
(structs.rs 1, enum_native.rs 7, math.rs 1, time.rs 5, complex.rs 9, proc.rs 2, io.rs 1), none tied
to a row (counted from the tool outputs in the records, including those saved to separate files).

Not in the table (outside the waves' bug lists, not attributed here): b8cb7f3 fixes four panics found
by a scan of 2944 test outputs, and b0553a4 a panic of the kind check introduced by the conversion
itself (Integer#pow), i.e. a regression the check caught, not a defect of the old code.

Found later (not by removing `unsafe`; the `unsafe` named is the one whose SAFETY claim was false):

| found | repro (one line) | before | unsafe / assumption | found by |
|---|---|---|---|---|
| 11-22 | AOT, patch binding on: `fork` a child, each side then first-dispatches new bodies (`tools/aot/fork_patch_arena_1122.rb`; ruby-bench knucleotide) | **SIGSEGV** / `memory allocation of 1.3e14 bytes failed` / a child killed (`SARUBY_PATCH=0`, `--plain` fine) | runtime/astrust_loader.rs `Arena::grow`: the RW window as `&'static mut [u8]`, "only this Arena uses it". The memfd is `MAP_SHARED`, so after `fork` the child's copy of the Arena (same `used`) wrote the parent's code pages: two processes held the one `&mut`. Fixed: an Arena records its pid; a child drops the inherited one unused and binds into a memfd of its own | c-test (a crash of a benchmark run): backtrace (rsp truncated in patch-bound code outside every library), then a per-allocation pid trace: parent and child allocated the same addresses (`2026-11-22-saruby-knucleotide-segv`) |

Closed by construction and not reproduced (wave 3 and 4 READMEs): `clone_class` read the source
class through a `'static` alias while allocating; `Hash.ruby2_keywords_hash` iterated a `Vec` of
pairs across `hash_aset`; Rational / Complex / Time / Proc bodies held their Values only in a `Box`
during allocation; `thread::run_pass` iterated a Rust copy of the thread list while fibers ran;
`spawn_spec` kept pairs in a `Vec` across `to_str`; the JSON parser held a `&'static [u8]` into the
source String while running callbacks; the old global list of iterated hashes compared by address.
Object accessors (section 7): `vm::ancestors` returned a `&'static Vec` into the ClassData that the
next recomputation for the class dropped; no caller found that iterates it across such a
recomputation (now an `Rc<[Value]>`).

Performance defects the work uncovered (not bugs of behaviour): the "allocator accident"
(`__rust_alloc` inlined or not depending on module grouping under ThinLTO: nesteddata −12 % / +5 %
from moving functions between files; safe_heap.md, `ideas/rejected/saruby-legacy-shims-in-object-rs`),
and a store-forwarding stall from building a by-value payload on the stack and handing it to an
out-of-line allocator (strscan +30 % cycles on −3 % instructions; the same mechanism made gcchurn
460M -> 725M cycles at the pilot's merge on sp4, `2026-11-20-astrust-vs-astro/bisect_4a06252.txt`).

#### 2.4.1 The pre-fix shapes rebuilt on the safe API

Each shape written as a function against today's API (base `b08c665`, appended to the converted file,
type-checked with `cargo check --release --features gc-marksweep`, then removed; sources and output in
`T/results/bug_attribution/`). A second check with the rejected ones removed finished with no error or
warning, so "compiles" below is a clean check.

| row | pre-fix shape | `cargo check` |
|---:|---|---|
| 7 | `let v = c.str_bytes(slf);` then `expect_str(c, a[0])?`, then `v` used (chomp) | E0502 |
| 7 | `let v = c.str_bytes(slf);` then `get_pat(c, ..)?`, `c.ary_new()`, `c.str_new(&v[..])` (split) | E0502 ×3 |
| 24 | `let io = readable(c, slf)?;` (`&mut RIo`) then `num_to_long(c, ..)?`, then `fill(io)` | E0499 |
| 8 | `let d = sio_mut(c, slf);` (`&mut StrIO`) then `num_to_long(c, ..)?`, then `d.pos` | E0499 |
| 8 | `let d = sio(c, slf)?;` (a `Copy` snapshot) then `num_to_long(c, ..)?`, then read at `d.pos` | compiles |
| 2 | index mask copied out of `c.hash(hv)`, the table re-borrowed per probe step, `key_eql(c, ..)?` in between | compiles |
| 11 | length copied, `c.str_bytes(slf).get(i)` re-borrowed per step around `yield1(c, ..)?` | compiles |
| 10 | `c.str_len(slf)` before `num_to_long(c, ..)?`, the slice taken after with the old length | compiles |
| 20, 32 | new Strings collected into a `Vec<Value>`, then one more allocation (`c.ary_from_vec`) | compiles |
| 14 | `c.str_bytes(m)` on an argument whose kind is not checked | compiles (the kind check panics when it runs on a non-String, as in row 18) |

#### 2.4.2 What the attribution says

Of the 39 rows, the borrow checker found one at compile time (row 7: a receiver's bytes held across
`to_str` in `chomp`, `chomp!` and `split`), the safe accessors' run-time kind check found one (row 18,
a panic where the code had segfaulted before the conversion), and the other 37 were found in the
conversion work itself: 22 while reading the code being rewritten, 15 by tests run during it. None of the
five SIGSEGVs was found by a check firing; three of them (rows 14, 15, 17) were found reviewing the lines
that a grep for accessor calls listed in the converted files, the other two (9, 21) while rewriting the
module. Of the shapes rebuilt in 2.4.1 that keep a reference, a length or an index across a callback,
the borrow checker rejects three
(rows 7, 24, and 8 when the payload is held as `&mut`) and accepts four (row 8 as a `Copy` snapshot,
rows 2, 10, 11): a length, an index or a snapshot copied out of the borrow compiles and keeps the defect.
The rooting shapes (rows 20, 32) compile. Row 24's shape would have been rejected, but io.rs was written
anew, so the compiler never saw it.

## 3. The checked value stack against the raw one on sp4

Source: `2026-11-20-astrust-vs-astro` (`results_sp4_all_<commit>.tsv`, `results_sp4_long_1cdddb3.tsv`).
Each run builds one commit twice on sp4, default (`astrust-aot`: checked `stack.rs`, no `unsafe`)
and `FEATURES=raw_stack` (`astrust-unsafe-aot`: `stack_raw.rs`), and runs the 53 rubyharness
benchmarks 5 times each (long runs: 3), wall time including process start-up. Tables made by
`T/scripts/safe_vs_unsafe.rb` and `T/scripts/pivot.rb` (`T/results/safe_vs_unsafe_{aot,interp}.txt`,
`pivot_{aot,interp}.md`); a missing or failed cell aborts them.

The two builds differ only in the stack backend, but **not only in the stack's instructions**: a
different backend changes inlining and code placement throughout `libsaruby.so`, and the baked code
is compiled against it. A difference here is "this build vs that build", and its mechanism is
known only where a local instruction count or profile says so (section 4 has the instruction
counts at `f88d3e6`).

### 3.1 How many benchmarks differ beyond the min–max range, per commit

| commit | AOT: safe slower / overlap / safe faster | interp: safe slower / overlap / safe faster |
|---|---|---|
| d42aa0e | 28 / 24 / 1 | 21 / 29 / 3 |
| 4a06252 | 20 / 30 / 3 | 15 / 35 / 3 |
| bb5c4c3 | 26 / 25 / 2 | 8 / 30 / 15 |
| 261e998 | 25 / 25 / 3 | 20 / 28 / 5 |
| 4985d89 | 27 / 23 / 3 | 34 / 19 / 0 |
| e71359b | 21 / 29 / 3 | 23 / 30 / 0 |
| d1aaaa1 | 20 / 28 / 5 | 17 / 33 / 3 |
| a944ba4 | 17 / 32 / 4 | 23 / 28 / 2 |
| 9c001c8 | 15 / 33 / 5 | 33 / 19 / 1 |
| d1a3233 | 15 / 31 / 7 | 12 / 36 / 5 |
| 6cdf354 | 3 / 48 / 2 | 5 / 48 / 0 |
| 6cdf354 (second run) | 2 / 50 / 2 | 8 / 44 / 2 |
| de58efd | 12 / 34 / 8 | 21 / 30 / 3 |
| 1cdddb3 | 14 / 35 / 5 | 13 / 37 / 4 |
| 1cdddb3, long (~1 s per benchmark) | 20 / 19 / 15 | — |

These are counts of individual benchmarks, given only to show how the picture moved; the per-benchmark
values are below. From the second 6cdf354 run on, the AOT columns include a `_startup` row (a file
containing `nil`; it always overlaps), so they sum to 54. At 6cdf354 the ranges widened (the run had larger run-to-run spread), which is why
almost everything overlaps there.

### 3.2 Per benchmark, AOT, across commits

Cell: safe median / unsafe median − 1, in %; **S** = safe slower and the ranges do not overlap,
**F** = safe faster and the ranges do not overlap; no letter = ranges overlap.

| bench | d42aa0e | 4a06252 | bb5c4c3 | 261e998 | 4985d89 | e71359b | d1aaaa1 | a944ba4 | 9c001c8 | d1a3233 | 6cdf354 | 6cdf354s | de58efd | 1cdddb3 | 1cdddb3 long |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| ackermann | +9.4S | +6.9S | +6.5S | +6.0S | +8.6S | +4.3S | +4.3S | +4.7S | +15.2S | -1.2 | -2.6 | -0.6 | +0.7 | +4.8S | +4.6 |
| array_access | +5.8S | +3.7S | +5.9S | +5.3S | +6.3S | +3.9 | +2.2 | +0.7 | +2.2 | +0.7 | -1.6 | -0.8 | +0.0 | +0.0 | -2.4F |
| ary | +2.3S | -0.6 | -3.2F | -3.2F | +2.1 | -1.7F | -5.7F | -7.5F | +0.0 | -11.5F | -7.3F | -11.0F | -2.9 | +22.6S | +101.8S |
| aryidx | +2.8 | +0.0 | +2.6 | -2.6 | +0.0 | +0.0 | +0.0 | +2.7 | +0.0 | +0.0 | +0.0 | +13.6 | +0.0 | +0.0 | -3.3F |
| bignum | -1.5 | +1.5 | +1.5 | -1.5 | +0.0 | +1.5 | +0.0 | +1.5 | +3.0 | +0.0 | -1.8 | -1.9 | +0.0 | -2.3 | -1.6F |
| binary_trees | +4.9S | +4.7S | +2.8 | +9.8S | +11.6S | +9.6S | +10.5S | +6.1S | +9.5S | +2.2S | +8.1 | +2.6 | +0.0 | -1.5 | -1.4F |
| bitops | +0.0 | +0.9 | +0.0 | +0.9 | -1.0 | +3.0S | +3.0S | +3.0S | +4.0S | +4.0S | +2.3 | -1.1 | -3.7F | +0.0 | +0.3S |
| block | +3.5S | +2.7S | +1.7 | +1.9S | +4.1S | +2.0 | +4.1S | +2.0S | +0.0 | +2.0 | +8.6 | +5.6 | +0.0 | +0.0 | +0.1 |
| block_yield_kernel | +2.1 | +2.1 | +0.0 | +2.2 | +3.6S | +3.6S | +3.5S | +2.4 | +0.0 | -1.9 | -2.6 | +7.7 | -3.8 | +4.3 | +0.6 |
| casewhen | +6.2S | +2.4 | +2.4S | -1.2 | +1.4S | +2.9 | +1.4 | +4.3 | +2.9S | +1.4 | +3.6 | -6.8 | +7.0S | +7.3S | +9.9S |
| closures | +1.1 | +1.4 | +5.9S | +1.1 | +13.1S | +2.8S | +10.6S | +1.9S | +3.2S | +5.7S | +5.7S | +5.0S | +8.1S | +0.8 | +1.9S |
| cmpsort | +3.6 | +5.2 | +5.3S | +6.2S | +3.9S | +3.9 | +4.7S | +5.5S | +4.6S | +3.9S | +9.6 | +4.4 | +7.8S | +4.9S | +7.5S |
| collatz | +6.0S | +0.0 | -1.2 | +1.2 | +2.8S | +0.7 | -18.4F | -19.4F | -18.6F | -18.4F | -20.2F | -19.3F | -22.4F | -19.7F | -22.0F |
| exception | +4.5 | +1.4 | +3.0 | +4.5 | +4.8S | +4.8S | -1.5 | -1.5 | +0.0 | +1.6 | +4.3 | +0.0 | +2.7 | +2.7 | +3.8S |
| fannkuch | +0.0 | +1.1 | +2.1S | +0.0 | +1.3 | +0.0 | +0.0 | +0.0 | +1.3 | -1.3 | -1.5 | +0.0 | +0.0 | +0.0 | -1.0 |
| fib | +9.3S | +4.1S | +5.4S | +5.0S | +3.7S | +3.0S | +0.7 | +2.2 | +1.5S | -3.4F | -2.9 | -3.8 | -5.2F | -6.6F | -7.3F |
| floatcalc | +0.0 | +3.8S | +4.9S | +1.0 | +0.0 | +0.0 | +0.0 | +0.0 | +0.0 | +0.0 | +8.6 | +1.2 | +1.4S | -1.4 | +2.3S |
| gc_bigobj | +0.0 | +1.5 | +1.5 | +0.8S | +0.8 | +1.6 | +1.6 | +1.6 | +0.0 | +1.6 | -0.9 | +0.9 | +0.0 | +0.0 | +0.4 |
| gc_wb | +2.7 | -1.2 | +0.0 | -2.5 | +1.4 | +0.0 | +1.7 | -1.7 | +0.0 | +1.7 | -2.2 | +7.0 | +0.0 | +0.0 | -0.8 |
| gcchurn | +6.1S | -1.2F | +3.2S | +3.1S | +2.5S | +0.0 | +2.6 | +0.0 | +0.0 | -1.3 | +5.2 | +0.0 | +0.0 | +2.2 | -0.7F |
| gcd | +8.7S | +7.7S | +8.2S | +7.5S | +7.1S | +6.7S | +7.2S | +7.2S | +6.7 | +9.9S | +11.9S | +12.9S | +11.9S | +3.6S | +3.5S |
| gen_gc | +2.0S | +0.0 | +1.4 | +1.4 | +0.7 | +0.8 | +0.0 | +0.8 | +0.0 | +0.0 | +0.0 | -2.5 | +1.0 | +1.0 | +1.4S |
| hash | +5.7S | +0.5 | +1.0 | +0.0 | +0.0 | +0.7 | -1.4 | +0.0 | -1.4 | +0.0 | -1.6 | +2.6 | -0.9 | +0.0 | +0.2 |
| hashiter | -0.9 | +1.9 | +1.0 | +9.0S | +10.5S | +10.5S | +11.6S | +10.6S | +1.6 | +3.2 | +6.4 | +0.0 | +0.0 | +12.5S | +18.1S |
| intdiv | +1.0 | +0.0 | -1.1 | -1.1 | +2.4 | +2.5 | +1.2 | +1.2 | -4.7F | -2.4F | -1.4 | -5.6 | +1.8 | -1.9 | -0.3 |
| iterators | +2.3S | +5.9S | +0.0 | +2.6S | +2.3S | +2.8S | +4.2S | +2.9S | +3.5S | +5.9S | +10.3 | +6.9 | +6.7S | +1.8 | +3.2S |
| ivar | +10.6S | +7.1S | +4.2S | +6.0S | +0.7 | +5.5S | +3.8S | +4.6S | +5.4S | +3.1 | +4.3 | +2.6 | +0.9S | -3.6F | -3.9F |
| kwargs | +4.4S | +2.1 | +0.5 | +1.1S | +3.5 | +4.7S | +2.9S | +2.8 | +1.4 | +4.2S | -3.3 | +0.0 | +6.7S | +2.4S | +3.6S |
| mandelbrot | +0.6 | +1.7S | +1.1S | -2.8F | +1.2 | +0.6 | +1.2 | +0.6 | +1.2 | +1.2S | -3.1 | +3.9 | +0.0 | +1.4S | +1.6S |
| mapreduce | +1.4 | +1.9 | +3.4S | +5.8S | +9.3S | +8.0S | +8.0S | +9.1S | +0.9 | +1.8S | +3.0 | +2.0 | +1.1 | +1.2 | +0.0 |
| mathfn | +5.3S | +5.9S | +6.0S | +4.5S | +4.7S | +2.3S | +1.8S | +1.8 | +2.7S | +2.7S | -2.0 | +4.1 | +2.2 | +2.2S | +2.7S |
| method_call | +4.5S | +4.5S | +4.5S | -4.5F | -4.5F | -4.9F | -5.5F | -4.9F | -4.4F | -5.5F | -5.9 | -3.6 | -5.5F | -6.1 | -6.2F |
| methodchain | +2.1S | +0.5 | +1.5S | +8.4 | +12.4S | +10.7S | +7.6S | +7.6S | +3.0S | +0.0 | +0.0 | -1.2 | +1.4 | +2.9S | -1.2 |
| nbody | +4.8S | +6.3S | +5.0S | +2.7 | +3.9S | +8.2S | +7.4S | +8.3S | +6.6S | +6.6S | +5.5 | +11.7 | +7.4S | +7.4S | +8.4S |
| nested_loop | +3.0 | +0.0 | +0.0 | -0.9 | -0.9 | +0.0 | +0.9 | -0.9 | -0.9 | +0.0 | +1.1 | -3.2 | -9.1F | +0.0 | +0.0 |
| nesteddata | +8.5S | +1.4 | +0.0 | -2.7 | +1.4 | +3.0S | +0.0 | +3.0 | +3.0 | +3.0 | +0.0 | +1.9 | +2.4 | +2.4 | -1.0F |
| object | +0.0 | +3.5S | +3.7S | +4.8S | +4.3S | +5.6S | +3.3 | +4.5S | +6.7S | +3.4S | +5.3 | +2.7 | +1.5 | +1.6S | +1.2S |
| poly | +4.9S | +7.2S | +5.5S | +4.0S | +4.5S | +5.4S | +5.4S | +6.2 | +4.5S | +4.4S | +4.0S | +3.0 | +4.6S | +4.9S | +4.2S |
| rangeeach | +2.6 | +2.6 | +1.3 | +1.4 | +2.9S | +0.0 | +2.9 | +1.5S | -2.3 | +0.0 | -15.6 | +10.7 | -5.9 | +0.0 | +0.1 |
| render_span_kernel | -0.5 | +4.6S | +2.0S | +3.4S | +1.2S | +3.0 | -0.7 | +0.0 | -1.4 | +0.0 | +3.0 | +0.0 | -0.8 | +0.0 | +5.2S |
| send | +10.9S | +3.3S | +2.6S | +6.5S | -3.5F | -4.6F | -5.4F | -3.1 | -4.6F | -4.6F | -5.2 | -6.8 | -4.6F | -5.5F | -5.7F |
| sieve | +0.0 | +1.2 | +3.8S | +1.2 | +1.3 | +1.3 | -2.5 | +1.3 | -1.3 | +1.3 | +0.0 | -1.5 | +0.0 | +0.0 | +0.4 |
| sort | +2.2S | +2.2S | +2.2 | +0.0 | +2.2S | +2.2 | +1.1 | +2.2 | -1.1 | +1.1 | +1.2 | -3.7 | -1.5 | -1.5 | +0.0 |
| sprintfb | +6.0S | -2.5F | +3.9S | +1.3 | +1.3 | +5.5S | -4.6F | +2.1 | +2.1 | +4.1S | +2.3 | +7.0 | +15.1S | +0.8 | +0.3S |
| str | +1.9 | +3.9 | +2.6 | +3.2S | +0.0 | +0.0 | -0.7 | +0.7 | +1.4 | -1.4F | -1.5 | -2.3 | -1.7F | -0.9 | -0.6F |
| strcmp | +0.0 | +1.8 | +0.0 | +1.8 | +1.8 | +0.0 | +0.0 | +0.0 | +3.6 | +0.0 | +15.4 | -7.0 | +0.0 | +0.0 | -0.2 |
| strfmt | -3.2F | +0.0 | -1.2 | +4.9S | +0.0 | +0.6 | -0.6 | -0.6 | +0.6 | -0.6 | +2.9 | -2.8 | +0.0 | -2.2F | -1.8F |
| strops | +3.9S | +0.0 | +3.9S | +4.8S | +4.1S | +1.6 | +1.7S | -0.8F | -2.7F | +0.9 | -1.1 | +1.1 | +0.0 | -1.2 | -0.7F |
| strscan | +0.9 | +0.0 | +0.0 | +0.9S | +1.8 | +1.9 | +1.9 | +1.0 | +0.9 | -0.9 | -1.1 | -4.1 | -1.2 | -1.2 | -0.2 |
| structacc | +5.1S | +5.3S | +8.5S | +7.6S | +8.2S | +7.3S | +5.4S | +7.2S | +9.2S | +9.1S | +16.7 | +6.2 | +9.8S | +7.7S | +13.3S |
| tak | +19.4S | +10.0S | +11.8S | +8.8S | +4.3S | +3.6 | +3.2S | +3.6S | +3.6 | -0.9 | -1.8 | +0.9 | -1.4F | -0.5 | -0.4F |
| while | +0.0 | -1.1 | +0.0 | -1.1 | +0.0 | +0.0 | +0.0 | +0.0 | +1.2 | +0.0 | +3.0 | +0.0 | +0.0 | +0.0 | -1.0 |
| while2 | +10.4S | -10.6F | -13.6F | +0.0 | -10.4F | +0.0 | +0.0 | +0.0 | +1.7 | +0.0 | +0.0 | +9.8 | +3.2 | +0.0 | +0.0 |

The interpreter's table is `T/results/pivot_interp.md`.

### 3.3 The long run (1cdddb3, ~1 s per benchmark, 3 runs each)

Median [min–max] wall seconds.

| bench | safe (checked) AOT | unsafe (raw_stack) AOT | safe / unsafe − 1 | ranges |
|---|---|---|---:|---|
| _startup | 0.009 [0.008-0.010] | 0.009 [0.008-0.009] | +0.0% | overlap |
| ackermann | 1.010 [0.995-1.013] | 0.966 [0.962-1.049] | +4.6% | overlap |
| array_access | 0.949 [0.948-0.955] | 0.972 [0.956-0.974] | -2.4% | safe faster |
| ary | 1.027 [1.013-1.033] | 0.509 [0.507-0.510] | +101.8% | safe slower |
| aryidx | 0.321 [0.321-0.323] | 0.332 [0.331-0.333] | -3.3% | safe faster |
| bignum | 0.853 [0.849-0.854] | 0.867 [0.860-0.871] | -1.6% | safe faster |
| binary_trees | 0.918 [0.916-0.922] | 0.931 [0.930-0.934] | -1.4% | safe faster |
| bitops | 1.009 [1.008-1.010] | 1.006 [1.005-1.007] | +0.3% | safe slower |
| block | 0.998 [0.997-1.000] | 0.997 [0.997-0.997] | +0.1% | overlap |
| block_yield_kernel | 0.979 [0.979-0.980] | 0.973 [0.966-0.986] | +0.6% | overlap |
| casewhen | 1.003 [1.002-1.004] | 0.913 [0.912-0.913] | +9.9% | safe slower |
| closures | 0.968 [0.966-0.974] | 0.950 [0.948-0.955] | +1.9% | safe slower |
| cmpsort | 0.990 [0.983-0.995] | 0.921 [0.919-0.921] | +7.5% | safe slower |
| collatz | 1.024 [1.024-1.026] | 1.312 [1.308-1.314] | -22.0% | safe faster |
| exception | 0.845 [0.843-0.846] | 0.814 [0.814-0.816] | +3.8% | safe slower |
| fannkuch | 1.010 [1.009-1.032] | 1.020 [1.007-1.024] | -1.0% | overlap |
| fib | 0.994 [0.994-0.995] | 1.072 [1.071-1.074] | -7.3% | safe faster |
| floatcalc | 1.051 [1.051-1.051] | 1.027 [1.025-1.028] | +2.3% | safe slower |
| gc_bigobj | 0.709 [0.709-0.711] | 0.706 [0.705-1.514] | +0.4% | overlap |
| gc_wb | 0.765 [0.760-0.766] | 0.771 [0.766-0.777] | -0.8% | overlap |
| gcchurn | 0.945 [0.945-0.947] | 0.952 [0.951-0.953] | -0.7% | safe faster |
| gcd | 0.948 [0.948-0.949] | 0.916 [0.916-0.917] | +3.5% | safe slower |
| gen_gc | 0.865 [0.864-0.866] | 0.853 [0.851-0.855] | +1.4% | safe slower |
| hash | 0.950 [0.947-0.951] | 0.948 [0.946-0.948] | +0.2% | overlap |
| hashiter | 0.997 [0.996-1.001] | 0.844 [0.843-0.845] | +18.1% | safe slower |
| intdiv | 1.025 [1.024-1.026] | 1.028 [1.019-1.048] | -0.3% | overlap |
| iterators | 1.000 [0.999-1.001] | 0.969 [0.969-0.970] | +3.2% | safe slower |
| ivar | 0.976 [0.974-0.977] | 1.016 [1.016-1.018] | -3.9% | safe faster |
| kwargs | 0.998 [0.998-1.000] | 0.963 [0.962-0.963] | +3.6% | safe slower |
| mandelbrot | 0.948 [0.947-0.949] | 0.933 [0.932-0.934] | +1.6% | safe slower |
| mapreduce | 0.583 [0.582-0.588] | 0.583 [0.583-0.583] | +0.0% | overlap |
| mathfn | 0.997 [0.996-1.000] | 0.971 [0.968-0.975] | +2.7% | safe slower |
| method_call | 1.035 [1.035-1.035] | 1.103 [1.103-1.104] | -6.2% | safe faster |
| methodchain | 0.925 [0.919-0.928] | 0.936 [0.927-0.938] | -1.2% | overlap |
| nbody | 1.002 [1.002-1.003] | 0.924 [0.923-0.924] | +8.4% | safe slower |
| nested_loop | 1.009 [1.008-1.012] | 1.009 [1.009-1.010] | +0.0% | overlap |
| nesteddata | 0.905 [0.904-0.912] | 0.914 [0.914-0.918] | -1.0% | safe faster |
| object | 1.007 [1.003-1.009] | 0.995 [0.994-0.999] | +1.2% | safe slower |
| poly | 1.007 [1.007-1.046] | 0.966 [0.963-0.967] | +4.2% | safe slower |
| rangeeach | 1.004 [1.003-1.006] | 1.003 [1.002-1.005] | +0.1% | overlap |
| render_span_kernel | 1.576 [1.535-1.593] | 1.498 [1.496-1.500] | +5.2% | safe slower |
| send | 1.042 [1.040-1.043] | 1.105 [1.105-1.105] | -5.7% | safe faster |
| sieve | 0.974 [0.974-0.976] | 0.970 [0.968-0.983] | +0.4% | overlap |
| sort | 0.649 [0.645-0.653] | 0.649 [0.648-0.649] | +0.0% | overlap |
| sprintfb | 1.010 [1.009-1.018] | 1.007 [0.999-1.008] | +0.3% | safe slower |
| str | 0.969 [0.966-0.971] | 0.975 [0.973-0.975] | -0.6% | safe faster |
| strcmp | 0.908 [0.908-0.910] | 0.910 [0.904-0.919] | -0.2% | overlap |
| strfmt | 0.983 [0.978-0.984] | 1.001 [0.999-1.003] | -1.8% | safe faster |
| strops | 0.981 [0.979-0.983] | 0.988 [0.985-0.988] | -0.7% | safe faster |
| strscan | 1.002 [1.000-1.004] | 1.004 [0.999-1.014] | -0.2% | overlap |
| structacc | 0.987 [0.986-0.995] | 0.871 [0.869-0.873] | +13.3% | safe slower |
| tak | 0.942 [0.941-0.942] | 0.946 [0.945-0.946] | -0.4% | safe faster |
| while | 0.997 [0.993-1.005] | 1.007 [1.004-1.017] | -1.0% | overlap |
| while2 | 0.994 [0.994-0.994] | 0.994 [0.994-0.995] | +0.0% | overlap |

### 3.4 Reading it

* **The call-path cost of the checked stack shrank and, on several call benchmarks, inverted.**
  tak: +19.4 % S at d42aa0e, +10.0 % S at 4a06252, +3.6 % at e71359b, −0.9 % at d1a3233, −0.4 % F in
  the long run. fib: +9.3 % S at d42aa0e, −3.4 % F at d1a3233, −7.3 % F in the long run.
  method_call: +4.5 % S up to bb5c4c3, −3.6 to −6.1 % from 261e998 on (F in 8 of those 11 runs;
  −6.2 % F long). send: +10.9 % S at d42aa0e, −3.1 to −6.8 % from 4985d89 on (−5.7 % F long). The changes in between are
  call-path work that bypasses `stage` on the hot calls: recursive self calls direct with a
  one-level unroll (d1a3233), self calls of parameterless methods in the caller's frame (e71359b),
  argument passing in registers (`@specin`). **Why the safe build ends up faster** on fib /
  method_call / send is not the instruction count: at `f88d3e6` the checked build still executes
  5.0 to 6.6 % more instructions on fib, tak, method_call and send in baked code (section 4.1), so the
  inversion is a property of the two binaries (code placement, front end), not of the stack design.
  Not traced further (no sp4 access for this work).
* **Persistently slower with the checked stack**, at nearly every commit and in the long run:
  gcd (+3.5 % S long; +3.6 to +12.9 % in the short runs), poly (+4.2 % S long), nbody (+8.4 % S),
  structacc (+13.3 % S), cmpsort (+7.5 % S), casewhen (+9.9 % S), mathfn (+2.7 % S), kwargs
  (+3.6 % S), iterators (+3.2 % S). A candidate mechanism, not checked on sp4: cfunc and block calls
  per iteration, whose arguments the checked stack stages with checks and copies out of the Cell
  slots (`with_args`, safe_stack.md "cfuncs"). Section 4.1 (AOT, `f88d3e6`): the raw build executes
  fewer instructions on structacc (−13.4 %), nbody (−5.5 %), poly (−5.3 %), mathfn (−5.3 %), gcd
  (−3.4 %), casewhen (−3.3 %), so there time and instructions agree in sign; on iterators they do not
  (raw +12.9 % instructions, yet the checked build is slower on sp4); kwargs and cmpsort were not
  counted.
  hashiter (+18.1 % S long, +12.5 % S short at 1cdddb3, ≈0 at de58efd) moves by more than any
  stack mechanism explains between neighbouring commits: placement suspected, not checked.
* **Persistently faster with the checked stack**: collatz, −18 to −22 % F from d1aaaa1 on, including
  the long run (1.024 s vs 1.312 s). Not explained; the inline builtins (`n.even?` inlined at baked
  sites) arrived at d1aaaa1. Section 4.1 confirms it in instructions: at `f88d3e6` the raw build
  executes 28.0 % more on collatz AOT (and +18.2 % on block, +12.9 % on iterators), all inside the
  baked code; why the raw backend's baked loop is longer was not traced.
* **ary long +101.8 % S** (1.027 s vs 0.509 s) is not the stack: it is glibc heap trimming (page
  faults), which depends on where a long-lived chunk lands and flips with a one-line change to the
  program (`2026-11-21-saruby-sp4-regressions`, section 1). The `mallopt(M_TOP_PAD)` fix (f495c0d)
  came after 1cdddb3.
* Interpreter: at 1cdddb3, 13 slower / 37 overlap / 4 faster; fib +0.6 %, tak −0.9 %, method_call
  +4.7 %, send +3.1 % (all overlapping), collatz +8.1 % S.
* The ledger entry `shipped/astrust-checked-stack-default` (the checked stack made the default at
  5453803, "trust first"; numbers from the d42aa0e run) quotes tak 0.299 -> 0.357 s, fib 0.151 -> 0.165, ivar 0.199 -> 0.220,
  send 0.147 -> 0.163 s; at 1cdddb3 those are tak −0.5 %, fib −6.6 % F, ivar −3.6 % F, send −5.5 % F.
  Its "覆る条件" (cost recovered by a safe means) is met for these four by the call-path work;
  for gcd / poly / nbody / structacc / cmpsort / casewhen it is not.

## 4. Gaps measured for this document

All on the development machine (not sp4: ssh was out of scope), against `f88d3e6` built the same way.
Each variant is a scratch branch with one change, never merged; the patches are in `T/patches/`.
Method: `T/scripts/perf.sh` / `perf2.sh` (per benchmark: bake once per build, then interpreter and
AOT runs with the builds interleaved in each of 5 repetitions under `perf stat -e
cycles:u,instructions:u`; minimum, and for cycles the spread (max − min) / min). **Load average 11 to
18 during the first run and 18 to 32 during the second**, so the cycle columns (spreads up to 100 %)
cannot decide anything below about 20 %; only instruction counts are read here. Every build: `make`
and `make FEATURES=raw_stack` without warnings (`T/logs/make.*`).

| variant | change | `unsafe` (heap_audit total) |
|---|---|---:|
| base | `f88d3e6` | 365 |
| `raw` | the same tree built with `FEATURES=raw_stack` (unchecked value stack) | 365 (+5 compiled) |
| `val` | `Value::fix` / `sym` / `flonum` / the constants via `NonZeroU64::new(..)` (`unwrap` / `map`, const-evaluated for constants); `LitCache.v: Option<Value>` instead of `u64` (node.def's five literal caches read `if let Some(v) = lc.v`) | 352 |
| `atomic` | `static mut REFINEMENTS_ON` and `rt::BOOTED` as `AtomicBool` (Relaxed) | 355 |
| `cfunc` | `CallCache.cfunc: Option<CFunc>` instead of `usize` + `transmute` | 363 |
| `vmchk` | `vm()`, `c.vm()`, `c.vm_mut()`, `gc::heap()` with `unwrap()` instead of `unwrap_unchecked()` (no `unsafe` removed: the check alone) | 365 |
| `rs29b` | the safe SBuf of branch `sbuf-safe-rs29` (a7a1f7e) cherry-picked onto `f88d3e6` (one conflict in string_methods.rs, resolved to the branch's form) | 355 |
| `nokalloc` | base run with `SARUBY_NO_KALLOC=1` (the system allocator instead of kalloc's `unsafe impl GlobalAlloc`) | (13 fewer would be needed) |

### 4.1 Call path: instructions (M, minimum of 5; change against base)

`T/results/perf.callpath.1` (raw), `perf.callpath.1.compact.md`; cycles with spreads in
`perf.callpath.1.compact_cyc.md`.

| bench | mode | base ins (M) | raw ins | val ins | atomic ins | cfunc ins | vmchk ins |
|---|---|---|---|---|---|---|---|
| fib | interp | 6723.1 | -7.0 % | +0.0 % | +0.0 % | +0.0 % | +0.0 % |
| fib | aot | 1894.7 | -5.2 % | +0.0 % | -0.0 % | -0.0 % | +0.0 % |
| tak | interp | 13675.5 | -7.7 % | +0.0 % | +0.0 % | +0.0 % | +0.0 % |
| tak | aot | 3773.5 | -6.6 % | +0.0 % | +0.0 % | +0.0 % | +0.0 % |
| ackermann | interp | 11332.7 | -6.3 % | +0.0 % | +0.0 % | +0.0 % | +0.0 % |
| ackermann | aot | 2820.6 | -4.9 % | +0.0 % | +0.0 % | +0.0 % | +0.0 % |
| method_call | interp | 13966.3 | -7.5 % | +0.0 % | +0.0 % | +0.0 % | +0.0 % |
| method_call | aot | 3020.8 | -5.0 % | +0.0 % | +0.0 % | +0.0 % | +0.0 % |
| send | interp | 11386.0 | -5.5 % | +0.0 % | +0.0 % | +0.0 % | +0.0 % |
| send | aot | 1970.9 | -6.1 % | +0.0 % | +0.0 % | +0.0 % | +0.0 % |
| block | interp | 3030.9 | -5.4 % | +0.0 % | +0.0 % | +0.0 % | +0.0 % |
| block | aot | 410.8 | +18.2 % | +0.0 % | +0.0 % | +0.0 % | +0.0 % |
| iterators | interp | 8218.4 | -5.6 % | +0.0 % | +0.0 % | -0.0 % | +0.0 % |
| iterators | aot | 1163.4 | +12.9 % | +0.0 % | +0.0 % | +0.0 % | +0.1 % |
| object | interp | 3388.4 | -5.1 % | +0.0 % | +0.0 % | +0.0 % | -0.2 % |
| object | aot | 1300.6 | -4.0 % | +0.0 % | +0.0 % | -0.3 % | -0.9 % |
| closures | interp | 3202.1 | -7.1 % | +0.0 % | +0.0 % | -0.5 % | +0.8 % |
| closures | aot | 1814.3 | -7.5 % | +0.0 % | +0.0 % | -0.5 % | +1.5 % |
| ivar | interp | 11568.8 | -5.8 % | +0.0 % | +0.0 % | +0.0 % | +0.0 % |
| ivar | aot | 2485.2 | -6.4 % | +0.0 % | +0.0 % | +0.0 % | +0.0 % |
| gcd | interp | 12120.1 | -6.4 % | +0.0 % | +0.0 % | +0.0 % | -0.0 % |
| gcd | aot | 3407.5 | -3.4 % | +0.0 % | +0.0 % | +0.0 % | +0.2 % |
| poly | interp | 4796.4 | -3.8 % | +0.2 % | +0.5 % | +0.0 % | +0.1 % |
| poly | aot | 2079.4 | -5.3 % | +0.0 % | +1.2 % | +0.0 % | +0.3 % |
| structacc | interp | 4617.9 | -9.0 % | +0.0 % | +0.0 % | +0.0 % | -0.0 % |
| structacc | aot | 1449.2 | -13.4 % | +0.0 % | +0.0 % | +0.0 % | +1.5 % |
| nbody | interp | 5409.5 | -6.9 % | +0.0 % | +0.0 % | +0.0 % | +0.0 % |
| nbody | aot | 2348.9 | -5.5 % | +0.8 % | +0.0 % | +0.0 % | +0.0 % |
| mathfn | interp | 4091.5 | -7.5 % | +0.0 % | +0.0 % | +0.3 % | +0.1 % |
| mathfn | aot | 1881.0 | -5.3 % | -0.8 % | +0.0 % | +0.0 % | +0.8 % |
| intdiv | interp | 5211.9 | +0.0 % | +0.0 % | +0.0 % | +0.0 % | -0.2 % |
| intdiv | aot | 1232.9 | -0.0 % | +0.0 % | +0.0 % | +0.0 % | +1.9 % |
| while | interp | 10815.7 | +0.0 % | +0.0 % | +0.0 % | +0.0 % | +0.0 % |
| while | aot | 1470.6 | +0.0 % | +0.0 % | +0.0 % | +0.0 % | +0.0 % |
| nested_loop | interp | 10387.8 | +0.0 % | +0.4 % | +0.0 % | +0.0 % | +0.0 % |
| nested_loop | aot | 1825.6 | +0.0 % | +0.0 % | +0.0 % | +0.0 % | +0.0 % |
| casewhen | interp | 5525.6 | -3.6 % | +0.0 % | +0.0 % | +0.0 % | +0.0 % |
| casewhen | aot | 938.4 | -3.3 % | +0.0 % | +0.0 % | +0.0 % | +0.0 % |
| collatz | interp | 9234.7 | -4.9 % | +0.1 % | +0.0 % | +0.2 % | +0.5 % |
| collatz | aot | 1020.7 | +28.0 % | +0.0 % | +0.0 % | +0.0 % | +0.0 % |
| str | interp | 4446.6 | -0.0 % | +0.0 % | +0.0 % | +0.0 % | +1.6 % |
| str | aot | 2566.7 | +0.9 % | +0.0 % | +0.0 % | +0.0 % | +2.8 % |
| strfmt | interp | 3193.1 | -2.6 % | +0.1 % | +0.0 % | +0.1 % | +1.6 % |
| strfmt | aot | 1925.8 | +1.4 % | +0.0 % | +0.0 % | +0.0 % | +1.8 % |

Reading, per variant:

* **`val`, `atomic`, `cfunc`: removing 13, 10 and 2 `unsafe` costs no instructions** (applied on master in 4.4, except the safe `fix` / `sym` / `flonum` of `val`). Of the 44 cells,
  `val` changes 6 (nbody AOT +0.8 %, mathfn AOT −0.8 %, nested_loop interp +0.4 %, poly interp +0.2 %,
  collatz / strfmt interp +0.1 %), `atomic` 2 (poly +0.5 % / +1.2 %), `cfunc` 6 (closures −0.5 % /
  −0.5 %, object AOT −0.3 %, mathfn interp +0.3 %, collatz interp +0.2 %, strfmt interp +0.1 %), all
  others ±0.0 %. These sizes are what an unrelated change does to this crate under ThinLTO with 16
  codegen units (safe_heap.md, "The String side": a change confined to symbol.rs moved sprintfb by
  +0.5 % and gen_gc by +2.0 %); they were not traced per function. poly's +1.2 % under `atomic` has
  a plausible mechanism (an atomic load is not merged or hoisted by LLVM, and poly misses its call
  caches, and a miss goes through `lookup_site` (vm.rs), which reads `REFINEMENTS_ON`), not checked.
  Compatibility: `T/results/outs.txt` (below).
* **`vmchk`: the check alone costs up to +2.8 %** (str AOT +2.8 %, intdiv AOT +1.9 %, strfmt
  +1.6 % / +1.8 %, closures and structacc AOT +1.5 %), and removes no `unsafe`: replacing an
  unchecked assumption by a check, spread over hundreds of call sites of a global, is not free in
  the way the context-borrowing design was (2 instructions per access, merged by the compiler after
  a kind test).
* **`raw`: the checked value stack still costs instructions at `f88d3e6`**: interpreter −2.6 to −9.0 %
  for the raw build on every call-carrying benchmark (fib −7.0 %, tak −7.7 %, method_call −7.5 %,
  structacc −9.0 %), AOT −3.3 to −13.4 % (fib −5.2 %, tak −6.6 %, method_call −5.0 %, send −6.1 %,
  structacc −13.4 %, ivar −6.4 %, gcd −3.4 %), ±0.0 % on while / nested_loop / intdiv. **But the raw
  build executes more on three AOT benchmarks: block +18.2 %, iterators +12.9 %, collatz +28.0 %**
  (and str / strfmt AOT +0.9 / +1.4 %).
  This settles part of section 3: (1) sp4's "safe faster" on fib, method_call and send at the latest
  commits is **not** fewer instructions — the checked build executes 5 to 6 % more there — so it is a
  property of the two binaries' code (placement, front end), not of the stack design; (2) collatz's
  −18 to −22 % on sp4 does follow instructions: the raw build runs 28 % more of them, all inside the
  baked code (98.0 % / 98.4 % of `instructions:u` samples in the patch-bound code of either build,
  `T/results/collatz_profile.{base,raw}.txt`). Why the raw backend's baked loop is longer was not
  traced (disassembly not compared); the backend is only the module that `lib.rs` selects, so the
  difference is in what LLVM makes of `stack_raw.rs`'s API inside the inlined builtin and block
  paths.

### 4.2 String buffer and allocator: instructions and cycles

`T/results/perf.sbuf_kalloc.1`, `perf.sbuf_kalloc.1.compact_cyc.md`. Load average 17.7 at the start,
32.1 at the end: the cycle columns are given for completeness and decide only where the spread is a
few percent (the CRuby `s_*` rows mostly are).

| bench | mode | base ins (M) | rs29b ins | nokalloc ins | base cyc (M, spread) | rs29b cyc (spread) | nokalloc cyc (spread) |
|---|---|---|---|---|---|---|---|
| str | interp | 4448.7 | +4.3 % | +0.2 % | 1327 (65 %) | +25.3 % (34 %) | +7.1 % (55 %) |
| str | aot | 2568.9 | +6.1 % | +0.3 % | 696 (78 %) | -0.8 % (81 %) | +13.6 % (60 %) |
| strops | interp | 2633.4 | +2.1 % | +8.9 % | 789 (36 %) | +2.9 % (7 %) | +12.5 % (17 %) |
| strops | aot | 1650.9 | +3.3 % | +14.2 % | 478 (19 %) | +25.0 % (9 %) | +15.9 % (10 %) |
| strcmp | interp | 685.4 | +0.6 % | +22.0 % | 200 (30 %) | -1.0 % (35 %) | +17.1 % (29 %) |
| strcmp | aot | 494.7 | +0.9 % | +30.6 % | 149 (44 %) | -2.7 % (25 %) | +21.6 % (27 %) |
| strfmt | interp | 3195.3 | +1.3 % | +0.2 % | 977 (26 %) | -1.2 % (16 %) | +0.0 % (14 %) |
| strfmt | aot | 1928.0 | +0.5 % | +0.4 % | 724 (4 %) | -21.0 % (32 %) | -2.0 % (15 %) |
| strscan | interp | 1617.9 | +9.4 % | +1.1 % | 513 (24 %) | -3.1 % (13 %) | -6.7 % (14 %) |
| strscan | aot | 1607.4 | +9.4 % | +1.0 % | 470 (25 %) | +7.0 % (46 %) | +2.0 % (20 %) |
| sprintfb | interp | 3239.0 | +1.2 % | +18.4 % | 1156 (44 %) | -4.2 % (30 %) | +13.5 % (41 %) |
| sprintfb | aot | 2563.3 | +1.6 % | +23.3 % | 929 (20 %) | -9.5 % (20 %) | +10.0 % (46 %) |
| gcchurn | interp | 4146.0 | +0.4 % | +0.1 % | 1385 (27 %) | +6.6 % (17 %) | +9.3 % (40 %) |
| gcchurn | aot | 990.3 | +1.6 % | +0.7 % | 405 (17 %) | -38.3 % (82 %) | -6.9 % (31 %) |
| gc_wb | interp | 1347.6 | +0.3 % | +0.5 % | 547 (38 %) | -9.4 % (42 %) | -7.2 % (45 %) |
| gc_wb | aot | 466.0 | +0.8 % | +1.5 % | 156 (52 %) | -7.8 % (68 %) | -3.7 % (52 %) |
| nesteddata | interp | 899.7 | -0.2 % | +98.6 % | 305 (51 %) | +1.6 % (27 %) | +83.6 % (29 %) |
| nesteddata | aot | 627.8 | -0.3 % | +141.4 % | 209 (43 %) | -4.3 % (62 %) | +104.3 % (32 %) |
| hash | interp | 5467.6 | +0.0 % | +0.1 % | 1901 (13 %) | +0.8 % (14 %) | -2.2 % (19 %) |
| hash | aot | 2127.8 | +0.0 % | +0.3 % | 634 (60 %) | +1.0 % (41 %) | +3.2 % (75 %) |
| binary_trees | interp | 2296.0 | +0.2 % | +0.3 % | 769 (28 %) | +7.0 % (44 %) | -3.2 % (28 %) |
| binary_trees | aot | 1235.9 | +0.3 % | +0.6 % | 540 (36 %) | -14.9 % (34 %) | -27.2 % (60 %) |
| gen_gc | interp | 2370.5 | +0.3 % | +31.2 % | 835 (58 %) | +2.5 % (53 %) | +18.3 % (62 %) |
| gen_gc | aot | 1486.5 | -0.2 % | +49.7 % | 629 (33 %) | -5.2 % (42 %) | +45.9 % (20 %) |
| s_concat__binary_concat_embedded | interp | 1848.9 | +0.7 % | +0.3 % | 818 (3 %) | +5.8 % (4 %) | +2.2 % (1 %) |
| s_concat__binary_concat_embedded | aot | 1427.3 | -0.2 % | +0.5 % | 660 (2 %) | +20.5 % (3 %) | -0.9 % (3 %) |
| s_concat__interpolation_same_heap | interp | 885.5 | +5.8 % | +59.7 % | 462 (3 %) | +12.2 % (4 %) | +44.2 % (5 %) |
| s_concat__interpolation_same_heap | aot | 551.9 | +6.5 % | +95.8 % | 293 (5 %) | +17.7 % (3 %) | +72.5 % (4 %) |
| s_split__to_words_10 | interp | 1616.8 | +7.8 % | +5.3 % | 867 (4 %) | +3.2 % (6 %) | +14.3 % (4 %) |
| s_split__to_words_10 | aot | 1588.0 | +7.9 % | +5.4 % | 871 (2 %) | -1.8 % (6 %) | +5.7 % (10 %) |
| s_rpartition__rpartition_10 | interp | 1516.0 | +4.0 % | +66.7 % | 1041 (2 %) | +7.8 % (5 %) | +59.0 % (6 %) |
| s_rpartition__rpartition_10 | aot | 1254.5 | +5.0 % | +80.2 % | 486 (86 %) | +22.3 % (66 %) | +108.6 % (59 %) |
| s_dup__uplus | interp | 1629.8 | +2.6 % | +0.4 % | 472 (40 %) | +2.4 % (13 %) | +3.8 % (18 %) |
| s_dup__uplus | aot | 742.4 | +5.8 % | +0.9 % | 338 (12 %) | -3.5 % (13 %) | +0.8 % (6 %) |
| s_dup__dup | interp | 2326.9 | +0.6 % | +37.1 % | 726 (4 %) | +1.2 % (4 %) | +28.3 % (8 %) |
| s_dup__dup | aot | 1761.8 | +0.9 % | +49.0 % | 531 (15 %) | +14.7 % (8 %) | +39.1 % (8 %) |
| s_upcase__upcase_10 | interp | 1553.0 | +1.0 % | +34.7 % | 775 (4 %) | +6.6 % (5 %) | +31.3 % (3 %) |
| s_upcase__upcase_10 | aot | 1240.9 | +1.3 % | +43.5 % | 342 (91 %) | +18.4 % (68 %) | +42.1 % (74 %) |
| s_slice__string_short | interp | 2701.2 | +1.5 % | +40.4 % | 773 (96 %) | +14.8 % (72 %) | +47.2 % (87 %) |
| s_slice__string_short | aot | 2391.0 | +1.8 % | +45.7 % | 706 (87 %) | +1.7 % (89 %) | +44.0 % (87 %) |

* **rs29b re-measured on `f88d3e6`**: instructions as on `0059037` (method-layer trial): str +4.3 % /
  +6.1 % (then +4.6 / +6.0), strscan +9.4 % / +9.4 % (+9.4 interp), to_words_10 +7.8 / +7.9 %
  (+7.9 AOT), interpolation_same_heap +5.8 / +6.5 % (+6.5 AOT), rpartition_10 +4.0 / +5.0 %, uplus
  +2.6 / +5.8 % (+5.7 AOT), strfmt +1.3 / +0.5 %, binary_concat_embedded +0.7 / −0.2 % (−0.2 AOT).
  The cost did not move with a day of other changes. Cycles: the earlier run's wins on
  binary_concat_embedded (−27 % / −40 %) did **not** reproduce: +5.8 % / +20.5 % here with spreads of
  2 to 4 %, on equal instructions; interpolation_same_heap +12.2 % / +17.7 % (spread 3 to 5 %),
  before +49 % / +62 %. Equal-instruction cycle differences of this size have moved sign between
  builds before (memsearch, −48 % from code placement, method-layer trial), so neither run decides the
  cycle side; sp4 would. The recommendation of the method-layer work stands: keep the unsafe SBuf.
* **kalloc is the most expensive `unsafe` to remove that was measured.** With the system allocator:
  nesteddata +98.6 % / +141.4 % instructions (+83.6 % / +104.3 % cycles), interpolation_same_heap
  +59.7 % / +95.8 % (cycles +44 % / +73 %, spreads 3 to 5 %), rpartition_10 +66.7 % / +80.2 %, gen_gc
  +31.2 % / +49.7 %, dup +37.1 % / +49.0 %, upcase_10 +34.7 % / +43.5 %, slice string_short +40.4 % /
  +45.7 %, strcmp +22.0 % / +30.6 %, sprintfb +18.4 % / +23.3 %, strops +8.9 % / +14.2 %; ±1.5 % on
  str, strfmt, strscan, gcchurn, gc_wb, hash, binary_trees, uplus, binary_concat_embedded (these
  allocate few malloc-backed buffers per iteration: inline strings, GC slots). The 13 `unsafe` of
  kalloc.rs are an `unsafe impl GlobalAlloc` and its methods; the safe alternative is to not have an
  allocator of one's own.

### 4.3 Compatibility of the variants

`T/scripts/outs.sh` (per file of rubyharness `t/hand`: stdout + stderr + exit status of base and the
variant, 20 s limit; a file whose base output differs between two base runs is "unstable"), output
`T/results/outs.txt`:

| variant | mode | files (patterns) | identical | different | unstable (base vs base) |
|---|---|---|---:|---:|---:|
| `val` | `--plain` | 243 (`str* sym* regexp* bignum* range* float* lit* integer*`) | 242 | 0 | 1 |
| `val` | AOT (baked, then run) | 213 (`str* regexp* range* bignum*`) | 212 | 0 | 1 |
| `atomic` | `--plain` | 77 (`refine* method* send* kernel*`) | 73 | 0 | 4 |
| `cfunc` | `--plain` | 114 (`method* send* kernel* refine* range*`) | 109 | 0 | 5 |
| `rs29b` | `--plain` | 154 (`str* sym* encoding* sprintf* format*`) | 154 | 0 | 0 |

The unstable files are the same in every row (`range_min_super_bt_label_1019`, `refine_vis_1017`,
three `kernel_*`). The benchmarks of 4.1 and 4.2 were not output-checked per variant by the perf
scripts; their outputs are covered for rs29b by the method-layer trial's 415-file check.

### 4.4 Applied on master (`d983d25`)

The zero-cost variants of 4.1 were re-applied by hand on master `d983d25` (patch binding, blop layer,
Encoding::Converter and TracePoint had landed since `f88d3e6`), together with four more of the same kind,
and re-measured. Raw data and scripts: `T/applied/` (`results/perf.{1,2,3}`, `perf.val*`, `perf.null1`,
`results/oc.run`, `logs/`). Development machine, load average 6 to 21 (written at the top and bottom of
each result file); instructions are read, cycles only where the spread is small.

| commit | change | `unsafe` (heap_audit) |
|---|---|---:|
| base `d983d25` | | 371 |
| val | `NIL` / `FALSE` / `TRUE` / `UNDEF` by a const fn; `LitCache.v: Option<Value>` | 362 |
| atomic | `REFINEMENTS_ON`, `rt::BOOTED` as `AtomicBool` (Relaxed) | 352 |
| cfunc | `CallCache.cfunc: Option<CFunc>` | 350 |
| gc | `gc::STACK_BASE`, `STRESS` as `AtomicUsize` | 348 |
| store | `aot::STORE` in a `thread_local!` `RefCell` (the store leaked, as before) | 343 |
| imm (= tip2) | `value_bake` emits `const { Value::imm(..) }` instead of `unsafe { Value::from_raw(..) }` | 342 (fib's SD crates 286 -> 270) |

Instructions (M, minimum of 5; each column is the commit against the build before it; gc and store were
built together), interpreter and AOT:

| bench | mode | base ins (M) | val | atomic | cfunc | gc + store | imm | tip2 vs base | base cyc (M, spread) | tip2 cyc (spread) |
|---|---|---:|---|---|---|---|---|---|---|---|
| fib | interp | 6725.7 | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | 1846 (16.1 %) | 2017 (9.7 %) +9.3 % |
| fib | aot | 1897.2 | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | 476 (5.9 %) | 482 (28.0 %) +1.3 % |
| tak | interp | 13678.1 | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | 4317 (33.2 %) | 4457 (55.5 %) +3.2 % |
| tak | aot | 3776.1 | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | 1049 (56.9 %) | 1062 (26.1 %) +1.3 % |
| method_call | interp | 13968.9 | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | 4035 (7.1 %) | 4070 (4.1 %) +0.9 % |
| method_call | aot | 3023.4 | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | 775 (3.3 %) | 777 (1.6 %) +0.3 % |
| send | interp | 11028.6 | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | 3167 (3.3 %) | 3213 (2.0 %) +1.5 % |
| send | aot | 1973.5 | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | 456 (13.4 %) | 452 (10.8 %) -0.9 % |
| ivar | interp | 11187.5 | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | 3204 (24.4 %) | 3182 (69.9 %) -0.7 % |
| ivar | aot | 2487.8 | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | 530 (4.8 %) | 531 (3.5 %) +0.1 % |
| object | interp | 3295.1 | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | 955 (2.2 %) | 924 (5.2 %) -3.2 % |
| object | aot | 1303.4 | ±0.0 % | ±0.0 % | -0.31 % | ±0.0 % | ±0.0 % | -0.31 % | 307 (12.8 %) | 309 (5.9 %) +0.8 % |
| block | interp | 3033.5 | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | 875 (4.4 %) | 820 (3.0 %) -6.3 % |
| block | aot | 413.4 | ±0.0 % | +0.07 % | ±0.0 % | -0.07 % | ±0.0 % | ±0.0 % | 96 (5.5 %) | 92 (13.4 %) -4.7 % |
| iterators | interp | 8236.9 | ±0.0 % | -0.24 % | ±0.0 % | +0.24 % | ±0.0 % | ±0.0 % | 2221 (87.2 %) | 2217 (19.3 %) -0.2 % |
| iterators | aot | 1166.0 | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | 266 (2.6 %) | 271 (6.4 %) +2.0 % |
| mapreduce | interp | 5270.0 | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | 1487 (36.9 %) | 1350 (38.6 %) -9.3 % |
| mapreduce | aot | 997.9 | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | 242 (51.3 %) | 231 (65.4 %) -4.6 % |
| ary | interp | 5262.9 | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | 1615 (4.1 %) | 1641 (4.0 %) +1.7 % |
| ary | aot | 1327.4 | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | 272 (11.0 %) | 276 (2.8 %) +1.3 % |
| str | interp | 4245.6 | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | 1097 (2.8 %) | 1128 (2.6 %) +2.9 % |
| str | aot | 2593.8 | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | 687 (2.2 %) | 612 (6.6 %) -10.9 % |
| hash | interp | 5288.1 | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | 1803 (8.6 %) | 1714 (7.1 %) -5.0 % |
| hash | aot | 2173.0 | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | 604 (4.1 %) | 612 (9.0 %) +1.3 % |
| nesteddata | interp | 893.9 | ±0.0 % | +5.08 % | ±0.0 % | -4.83 % | ±0.0 % | ±0.0 % | 282 (15.4 %) | 265 (15.0 %) -6.0 % |
| nesteddata | aot | 629.2 | ±0.0 % | +7.09 % | ±0.0 % | -6.63 % | ±0.0 % | ±0.0 % | 184 (9.7 %) | 187 (15.4 %) +1.5 % |
| gcchurn | interp | 4050.8 | ±0.0 % | +0.20 % | ±0.0 % | -0.20 % | ±0.0 % | ±0.0 % | 1313 (30.6 %) | 1258 (71.7 %) -4.2 % |
| gcchurn | aot | 991.0 | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | ±0.0 % | 242 (91.4 %) | 228 (93.3 %) -5.9 % |
| poly | interp | 4703.1 | ±0.0 % | +0.26 % | ±0.0 % | -0.26 % | ±0.0 % | ±0.0 % | 1422 (5.1 %) | 1390 (4.4 %) -2.3 % |
| poly | aot | 2052.0 | ±0.0 % | +0.60 % | ±0.0 % | -0.60 % | ±0.0 % | ±0.0 % | 450 (9.8 %) | 456 (5.0 %) +1.4 % |

* **tip2 against base: ±0.0 % on 29 of 30 cells; object AOT −0.31 % (from cfunc).** optcarrot (bake
  60 frames, checksum 59662 at 180 frames, per frame = (500-frame run − 200-frame run) / 300, 5 rounds):
  82.68M -> 82.63M instructions (−0.06 %), 24.16M -> 23.92M cycles with a 500-frame spread of 58.7 % /
  4.8 %: cycles cannot decide, and nothing indicates a cost. The cycles column moves −10.9 % to +9.3 % on
  equal instructions with spreads of the same size; on the cells with spreads below 5 % on both sides
  (method_call AOT +0.3 %, send interp +1.5 %, ary interp +1.7 %, str interp +2.9 %, block interp −6.3 %,
  poly interp −2.3 %) the signs are mixed. The machine cannot decide these; sp4 would.
* **atomic, taken alone, costs instructions that the next commit gives back exactly**: nesteddata
  +5.08 % / +7.09 %, poly +0.26 % / +0.60 %, gcchurn interp +0.20 %, iterators interp −0.24 %; gc + store
  undoes each to the digit. nesteddata's +45.0M is the same count that three builds of the `val`
  experiments below reach without any atomic, so it is a state of the code generator, not the cost of a
  Relaxed load. Measured on `f88d3e6` the same commit gave poly +0.5 % / +1.2 % (4.1).
* **Not applied: the safe `Value::fix` / `sym` / `flonum`** (−4). Each safe form is correct and leaves
  no check in the machine code, but it moves the register allocation of functions it is inlined into:
  `hash_set` keeps two arguments in callee-saved registers from its entry (+2 instructions per call:
  hash +0.57 % / +1.38 %, exactly 30.0M for 15M calls), and a safe `fix` alone moves nesteddata
  +5.0 % / +7.0 % and poly +0.26 % / +0.60 %. Bisection (`results/perf.val*`): constants alone (valC)
  and constants + literal caches (valG) are identical to base on every cell; constants + `sym` + `flonum`
  (valH) also; valG + valH together (valF) flip `hash_set`; `fix` alone (valE) carries all three
  effects; `NonZeroU64::new(..).unwrap()` and `NonZeroU64::MIN | bits` give the same hash count. A
  semantically null edit of `fix` that keeps the `unsafe` (an extra `#[inline(always)]` helper, null1)
  is identical to base, so the harness is deterministic, and the effect needs these particular IR
  changes; it was not traced further than `hash_set`'s prologue.
* **Not applied: `ConstCache.val: Option<Value>`** (−1). The hit test gains a `Some` check per constant
  read in the interpreter: hash +0.28 % (one instruction per read of `INNER`), str +0.28 %, gcchurn
  +0.20 %, object +0.12 %, ±0.0 % AOT. Patch: `T/applied/rejected/`.
* `imm` changes the baked code of 93 of 1217 SD functions of optcarrot in size, both ways (net +6208
  bytes, +0.3 % of `.text`; a second bake of base changes none), because the const block is less IR than
  an inlined `from_raw`; run time ±0.0 % on every AOT cell and −0.06 % on optcarrot.
* Compatibility (`T/applied/logs/compat.txt`, `cmp-*.txt`; tip2 against base, per file stdout + stderr +
  exit status): t/hand `--plain` 1832 files, 1829 identical, 0 different, 2 unstable (base differs from
  itself), 1 timeout; AOT (baked) against `--plain` on every 8th t/hand file: tip2 229 / 229 identical
  with patch binding and with `SARUBY_PATCH=0` (base: 230 / 230 both; a file was added to rubyharness
  between the runs); t/spec + t/method `--plain` 1237 / 1237; no "panicked at" in any output;
  `tools/reuse_test.sh` and `tools/aot_fork_test.sh` PASS; `make`, `make FEATURES=raw_stack`,
  `make GC=bitmap` without warnings; `heap_audit.rb` exit 0.

## 5. What remains unmeasured

Each item is a cost that this document states only by argument (section 1.3 marks them **A**).

| item | `unsafe` concerned | why not measured | cheapest next step |
|---|---:|---|---|
| frame records in an indexed arena owned by `Ctx` (the safe form of the frame chain) | 39 call path + 5 heap.rs, and `Ctx::frame()` | a redesign of every call path; the closest measured shape is the value stack's `emu_idx` (+16 to +89 % AOT instructions) | an emulation like the stack's step 1: frames in a `Vec` with `prev` as an index, only on the call path |
| `&'static` + `Cell` for method entries, crefs, iseqs | 36 + 11 + 16 (+12 in heap.rs) | many signatures (`*const MethodEntry` is in `CallCache`, `Frame`, `MKind`, the generated code) | iseqs first: `c.iseq_ref` already returns `&'static Iseq` |
| block pointers with lifetimes (H4) | 17 | changes the `CFunc` type of every builtin | — |
| nodes in an index arena (generated code) | 1131 generated + per-bake SD code | the node address is part of the design (laid-out trees, baked `root + offset`, patch holes) | — |
| removing the 449 legacy-accessor call sites (rt.rs, vm.rs, node.def) | the 10 unsound safe functions | large mechanical conversion; the pilot's per-access cost (2 instructions, 0 to +2.1 %) is the estimate | convert rt.rs's env / cref helpers first |
| `unreachable_unchecked` -> `unreachable!()` (rt.rs env 3, generated `DispList::nth`) | 3 + one per baked list | generator owned by other work | one generator line; count instructions |
| kalloc's TLS `asm!` via `thread_local!` | 2 | — | one build, the allocator benchmarks of 4.2 |
| fibers on OS threads | 23 | a different runtime | a micro benchmark of a thread handoff against the 403-cycle switch |
| sys.rs through `nix` / `rustix` | 67 | moves the `unsafe`, does not remove it | — |
| every cycle number of section 4 on sp4 | — | ssh out of scope for this task | rerun `perf.sh` / `perf2.sh` there with the snapshots |
| why the raw stack's baked code is longer on collatz / block / iterators (4.1), and why the checked build is faster in time on fib / method_call / send on sp4 with more instructions (3.4) | — | not traced | disassembly of the baked loop of collatz in both builds; sp4 `perf stat` with front-end counters |
| `sample/astrogre` (265 lines) and `runtime/gc` (34) | | out of scope here | — |

## 6. Findings for a paper

Measured, with the trial that holds the numbers:

1. **Everything that implements Ruby methods became safe code at a cost that is within the noise of
   unrelated changes, and the conversion paid for itself by finding bugs.** 52 files under
   `forbid(unsafe_code)`, the whole crate 651 -> 365. The design that made it cheap is the
   context-borrowing accessor (a payload reference borrows the interpreter context, so "a reference
   held across a GC point or a callback" is a borrow error): its run-time cost is a kind check, 0 to
   +2.4 % instructions per benchmark (pilot, wave 2). The interior-mutability alternative cost up to
   +96 % cycles (RefCell emulation). The forced restructuring was often a gain (hash −36 %, io_write
   −11 %, json −10 % instructions).
2. **What the conversion found** (table 2.4, with how each row was found): 39 behavioural defects, of
   which 5 SIGSEGVs, 1 undefined behaviour (`from_utf8_unchecked`), 3 GC rooting / use-after-free
   defects (MatchData#to_a with its Strings only in a Rust `Vec` during an allocation, IO.readlines'
   options Hash, marshal_dump + GC.start), 1 latent uninitialized field (boot's `MaybeUninit<Vm>`), and
   stale-reference-across-callback defects (a `&'static [u8]` used after `to_str` moved the buffer; hash
   probes across `eql?`). One was found by the borrow checker at compile time (chomp / split: the
   receiver's bytes held across `to_str`), one by the accessors' run-time kind check (Symbol#upcase),
   22 while reading the code being rewritten and 15 by tests run during the work; none of the SIGSEGVs
   was found by a check firing. Most were Ruby-level invalidation or compatibility defects, not
   memory-safety bugs Rust could name, and of seven pre-fix shapes rebuilt on the safe API that keep a
   reference, a length or an index across a callback, four compile (2.4.1).
3. **Surprisingly removable at zero instruction cost** (section 4): `Value` construction and the
   literal caches (13 `unsafe`), `static mut` flags as atomics (10; REDEF earlier, 2), function
   pointers in caches as `Option<fn>` (2). Rust's niche optimisation (`NonZeroU64`, `Option<fn>`,
   `Option<NonZero>`) makes the safe type the same word as the unsafe one.
4. **Surprisingly necessary, or expensive to remove**:
   * the allocator: kalloc's `unsafe impl GlobalAlloc` is worth +23 to +141 % instructions on
     allocation-heavy string and hash benchmarks (4.2);
   * the 24-byte small-string buffer: three safe layouts measured, the best still +4 to +9 %
     instructions on short-string benchmarks, stable across a day of other changes (4.2);
   * the value stack: the checked stack is the default (policy: trust first) and still executes
     3 to 13 % more instructions on call-heavy benchmarks at `f88d3e6` (4.1) — but on sp4 its wall
     time is equal to or below the raw stack's on fib, tak, method_call, send (3.2), and the raw
     stack is *slower* on collatz (−22 % for safe, long run) with 28 % more instructions. "unsafe is
     faster" did not hold as a rule: in the long run (53 benchmarks + start-up) 20 are slower with
     the checked stack, 15 faster, 19 overlap, by min–max range;
   * the fiber switch, dlopen / mmap of baked code, the patch loader's relocated holes, the GC's
     stack scan: no safe Rust exists; these are the irreducible kernel together with the 67 libc
     wrappers.
5. **Checks are not the cost; globals are.** Turning `unwrap_unchecked` into `unwrap` on the VM and
   GC globals (no `unsafe` removed) costs up to +2.8 % instructions, more than any of the three
   zero-cost removals; the context-borrowing design's checks merge with the type tests the code makes
   anyway.
6. **The count of `unsafe` is a weak metric.** 449 call sites of ten *safe* functions with unsound
   `&'static mut` signatures remain; part of one reduction (node.def 27 -> 16) was achieved by routing a
   dereference through such a function. The generated code carries 1131 `unsafe` (plus ~260 per
   baked program), all of the "node pointer" kind.
7. **Methodology**: with 16 codegen units, unrelated changes move instruction counts by ±0.5 to 2 %
   and cycles by more (an allocator inlining "accident" of −12 % / +5 %; placement effects of −48 %
   on equal instructions); a safety change must be measured against a control with one codegen unit
   or several builds, and cycle effects below ~5 % need a dedicated machine.

Assumed, not measured: the cost of a safe frame chain, of safe node graphs, of safe fibers (section 5).

## 7. Design note: the `&'static mut` accessors

Trial: `2026-11-22-saruby-static-mut` (inventory script, options, measurements, patch). Base
`b08c665`.

**Inventory.** Ten safe functions returned an unbounded `&'static mut`: object.rs `obj`, `rhash`,
`rclass`, `rproc`, `rmethod`, `rbig`, `rdata` (and the macro-made `rstr` / `rary` / `ivars_of`, not
seen by the audit's regex), vm.rs `vm()` and `Ctx::frame()`, gc.rs `heap()`. Call sites (hand-written
code, comments stripped): `vm()` 148, `gc::heap()` 20, `Ctx::frame()` 29, object accessors 272;
469 in all (rt.rs 189, vm.rs 164, node.def 32, object.rs 25, legacy.rs 14, gc.rs 12, str_legacy.rs
11, lib.rs 10, misc.rs 7, extras.rs 2, aot.rs / compile.rs / inline_builtin.rs 1 each).
Of 184 field reads through `vm()`, 142 are builtin class / module values (set by boot, `Copy`);
the writes are tables (`node_values`, `gen_ivars`, `mcache`, ...) and counters (`const_serial`,
`hierarchy_serial`, `sd_hits`, `next_object_id`). Hot: `const_serial` and `c_array` / `c_string`
/ `c_hash` in node.def, the immediates' classes in `class_of`, `mcache` on an inline-cache miss,
`send_cache` per `send`, `heap()` per allocation, `Ctx::frame()` per line record (`pos` is its only
written field).

**Aliasing that exists today** (UB under the Rust model, no visible bug): a caller's `let vm = vm()`
live across an allocation whose collection calls `vm()` again (`new_class`; `visit_roots`,
`visit_gen_ivars` with `gen_ivars.get_mut`); boot's `vmr: &mut Vm` live across `define_class`
(which calls `vm()`); the root scan's `&mut *vm.main_ctx` while the allocating code holds its
`c: &mut Ctx`. `Ctx::frame()` had one reference held across `&mut Ctx` calls (`srb_super` read
`f.blk` after `stack_check` / `make_block` / `block_from_value`).

**Options** (full table in the trial README):

| option | `unsafe` | sites | cost |
|---|---:|---:|---|
| (i) thread the context: `c.vm()` / `c.vm_mut()`, `c.frame()` / `c.frame_mut()`, `c.ary(v)`.. | −1 per removed global accessor, −7 object.rs when unused; heap.rs keeps 2 | 469 + `c` parameters for context-free callers (`class_of`, `new_class`, `lookup_method`, ...) | M: accessor free (slice below); kind checks 0 to +2.1 % (heap pilot) |
| (ii) `Cell` per `Copy` field, `RefCell` for tables off the hot path, `vm() -> &'static Vm` | 0 (−1 with a TLS owner) | ~150 field accesses + boot's 329 `vmr.` | A: `Cell` 0; `RefCell` a flag test per `mcache` miss / `send` |
| (iii) one owner with scoped borrows (`with_vm`, `with_heap`) | −1 each with TLS | 469 closures; the collector re-enters, so callers must release before allocating | A: flag + TLS per scope; `STORE` precedent ±0.0 % (cold) |
| (iv) raw pointer, only `&self` + `Cell` fields | 0 | as (ii); impossible for `Vec`-backed payloads | A: 0 for `Copy` fields |

Proposed end state for `vm()`: builtin classes in an immutable `&'static` table built by boot,
counters as `Cell<u64>`, tables through `c.vm_mut()`, `gen_ivars` out of the `Vm` the mutator
borrows (the collector needs it). For `gc::heap()`: private, behind gc.rs functions
(`gc::stats()`, `gc::set_disabled`) that do not re-enter.

**Applied: `Ctx::frame()`** as `frame(&self) -> &Frame` and `frame_mut(&mut self) -> &mut Frame`
(21 writers to `frame_mut`, 8 readers unchanged, `srb_super` reads `f.blk` first). `fn->&'static mut`
10 -> 9, `unsafe` 342 -> 343 (two bounded derefs instead of one unbounded). `libsaruby.so` code
identical except `srb_super`; instruction counts identical to the printed M on all 15 benches
interp and AOT, startup in the same range, optcarrot 82.63M -> 82.64M per frame (local machine,
shared: instructions only).

**Applied: the vm / heap split** (trial README, section "vm/heap split"; base master `0db5dd0`,
three commits). Counts are `heap_audit.rb`, base -> after.

1. `gc::heap()` private: `HEAP` and `heap()` are private to gc.rs; GC.count / GC.stat /
   GC.disable / ObjectSpace.count_objects and the exit stats call `gc::collections`, `stats`,
   `set_disabled`, `total_time`, `backend_name`. No `&mut Heap` leaves gc.rs, and none of the
   collector's callbacks (root scan, edges, finalize, Data visit / drop; `data_free_hooks` is
   empty) calls a gc.rs function, so the private `&'static mut` never has two live copies.
   Instruction counts identical. A closure form, `with_heap(|h| ..)` (no `fn -> &'static mut` at
   all), measured gcchurn AOT 988M -> 1212M (+23 %) and str 4242M -> 4314M interp (+1.7 %), two
   runs each, equal: rejected (cause not investigated then; found later: "Applied: the gc heap" below).
2. `vm() -> &'static Vm`; no `&mut Vm` exists anywhere. Every field is interior-mutable: builtin
   classes and the other Value roots `Cell<Value>` (written by boot and the lazy inits of
   TracePoint / StringScanner / Encoding::Converter / ..., read with `.get()`), counters and
   flags `Cell<u64 / bool / usize / i32>`, `MethodIds` `Cell` pointers, `send_cache`
   `[Cell<SendWay>]`, the 22 tables `RefCell` (globals, mcache, gen_ivars, node_values,
   frozen_lits, pinned, blop, ...). `Ctx::vm_mut`, boot's `vmr: &mut Vm` and
   `Global::set_vm(&mut Vm)` are gone. The collector borrows the tables it scans or edits
   (`gen_ivars` in mark and finalize), so a mutator holding a table borrow across an allocation
   panics instead of aliasing. The conversion found one such site: `srb_fstr` held
   `frozen_lits.get(s)` (the match scrutinee) across `str_new_enc`; it copies the lookup out
   first now. heap.rs `unsafe` 32 -> 30 (the `vm` / `vm_mut` derefs), total 379 -> 377, vm.rs
   `fn->&'static mut` 1 -> 0.
   *Not done as proposed above:* "tables through `c.vm_mut()`". The collector reads nearly every
   table as a root and is reached from context-free `gc::alloc` (114 call sites), so a borrow
   through the context would not exclude it; the tables it does not touch are used from
   context-free code (`lookup_method` and mcache) or would need a second `unsafe` deref to hand
   out `&mut`. `RefCell` costs a flag test, on paths off the per-call fast path except the mcache
   lookup of an inline-cache miss.
   *Main-context root scan:* no longer `&mut *main_ctx` beside the allocating code's
   `c: &mut Ctx`; it reads the stack bounds and the four Value fields by place (`visit_main_ctx`).
   That is still a read of memory a live `&mut Ctx` covers (UB under Tree Borrows when that
   reference is a protected argument). The full fix hands the collector the allocating context,
   i.e. allocation only through `Ctx::alloc` (the heap pilot's end state). Open (done for the
   allocations through `Ctx`: "Applied: the gc heap" below).
3. Object headers through heap.rs (`obj_klass`, `obj_set_klass`, `obj_flags`, `obj_set_flags`,
   `obj_clear_flags`) at the 39 sites of vm.rs / rt.rs / node.def / legacy.rs / object.rs /
   str_legacy.rs that used `obj(v)` only for klass / flags. `obj` sites 70 -> 31, old-accessor
   sites 452 -> 414. The payload accessors (`rclass` 94, `rary` 59, the remaining `obj` 31, ...)
   need `c` threaded into context-free vm.rs / rt.rs functions: not done.

Instructions (local machine, shared; M = 10^6 `instructions:u`; base / after, base and after
each run twice, equal): 13 of 15 benches within 2M (0.2 %) interp and AOT; nesteddata 907 -> 884
interp, 642 -> 619 AOT; poly 4700 -> 4706 interp (+0.13 %), 2049 -> 2055 AOT (+0.29 %; poly is a
megamorphic site, so presumably the mcache `RefCell` on each miss: not verified). Startup
`-e 'p 1'` (CODEGEN_UNITS=1, 5 runs) 15.417M -> 15.459M (+0.28 %). optcarrot 82.64M -> 82.64M
per frame, checksum 59662. Step 3 alone: no change beyond 2M on any bench.

**Applied: the object accessors** (trial README, section "object accessors"; base master
`1749107`, eight commits `9e7fed4` .. `9f3b879`). object.rs no longer defines `obj`, `rary`,
`rstr`, `rhash`, `rclass`, `rproc`, `rmethod`, `rbig`, `rdata`, `ivars_of`, `str_bytes` nor the
context-free Array API built on them; every site uses heap.rs's context-borrowing accessors, with
`c` threaded into the vm.rs / rt.rs functions that needed it (the class API: `&Ctx` for reads,
`&mut Ctx` for writes and allocation). Boot runs on the main context: `heap::boot_vm` only installs
the blank Vm, lib.rs makes the main `Ctx` and calls `builtins::boot(&mut c)`, so the builtins'
`init(vmr: &Vm)` became `init(c: &mut Ctx)` (`Ctx::vm` returns `&'static Vm`). `ancestors()`
returns an `Rc<[Value]>` from a `RefCell` cache instead of a `&'static Vec` into the ClassData.
vm.rs, rt.rs, legacy.rs, str_legacy.rs import `heap::no_legacy_body` (state "converted (object
accessors)" in `heap_audit.rb`: they still use `vm()` and the context-free allocators, which the
full `no_legacy` bans). Counts, base -> after: `fn->&'static mut` 8 -> 1 (gc.rs's private `heap()`),
object-accessor sites 233 -> 0, context-free Array API 27 -> 0, `unsafe` 377 -> 380 (object.rs
-7, rt.rs -3, heap.rs +9 and node.def +4: the env accessors below).

*Env access stays unchecked, now marked:* node_dvar / node_dasgn use
`Ctx::env_{up,get,set}_unchecked` (heap.rs, `unsafe fn`, rule H6: a frame's env slot holds an
Env). They borrow the context like every accessor, so the aliasing assumption of base's
`obj(env)` is gone; base's `unreachable_unchecked` kind assumption remains, as a marked `unsafe`.
The checked `c.env_*` cost block AOT +29 % and iterators AOT +25 % instructions (two `test` / `cmp`
and two branches per access).

*Bug found:* row 40 of 2.4 (`{**h}` and four other sites copied a Hash's pairs into a `Vec` across
`hash_aset`: use after free under GC; c-read). *Closed by construction:* `ancestors()`'s
`&'static Vec`, dropped by the next recomputation for the same class (no reachable case found).

Instructions (local machine, shared; M = 10^6, base 3 runs, after 2 runs, equal): 12 of 15
benches within 0.3 % interp and AOT (nesteddata -0.23 % / -0.32 % with `drop_body` per variant);
poly 4706 -> 4746 interp (+0.85 %), 2055 -> 2079 AOT (+1.17 %); object 3292 -> 3304 (+0.36 %),
1296 -> 1300 (+0.31 %); startup 15.459M -> 15.496M (+0.24 %). All of the excess comes with the
`rclass` step (`a1217f5`); per callgrind: `call_slow` +2.7 instructions per megamorphic call
(register moves and reloads on the poly-table hit path, no added operation), the ivar node +1 per
read (the out-of-line miss call needs `c`, kept in a second register), `new_object` +1 per object.
Analysed, not fixed: above the 0.3 % bound, kept on the branch for the decision.
optcarrot 82.64M -> 82.94M per frame (+0.36 %), checksum 59662. Checks (trial README): three builds
0 warnings, t/hand 0 differences, AOT vs `--plain` the same 7 address-only files as base, panic scan
and GC stress 0 panics, reuse / fork tests pass.

**Applied: the gc heap** (trial README, section "gc heap"; base master `4833f74`, two commits).
No function returns `&'static mut` any more (`fn->&'static mut` 1 -> 0).

*Why `with_heap(|h| ..)` cost +23 %* (reproduced: gcchurn AOT 986M -> 1210M, str 4241M -> 4313M
interp): the closure `|h| h.alloc(..)`, which holds the whole inlined allocation fast path, stayed
out of line in the AOT crate's code (`gc::alloc_with::<ary_new_small::{closure}>::{closure#0}`,
38M of callgrind's 56M difference at OUTER=10): a call per allocation, and the body built in the
caller's frame and copied with 16-byte loads, the stall `alloc_with` exists to avoid.
`#[inline(always)]` on the closure restores gcchurn (985M), but str stays +0.85 % interp / +1.4 %
AOT: `DISPATCH_node_str` +3 instructions per call, the inner `move || RObj { .. }` closure's
captures stored to the stack before the free-list test (base: on the slow path only). Not kept.

*The form kept:* `fn heap(_: &mut Scope) -> &mut Heap`, called as `heap(&mut Scope).f(..)`. The
borrow lasts as long as the temporary `Scope`, i.e. to the end of the statement, so no `&mut Heap`
is kept in a local, returned or stored (`let h = heap(&mut Scope); h.f()` is E0716). Within a
statement a second `&mut Heap` would need a gc.rs function called while a Heap method runs; the
only saruby code a Heap method runs is the GcLang callbacks (root scan, edges, finalize, Data
visit / drop) and `each_object`'s `f`, and none calls gc.rs. The compiler does not check that last
part, so debug builds do: `heap()` sets a flag that `Scope`'s `Drop` clears and panics if it is
already set. Dev build over t/hand, t/spec, t/method (3102 files) and a GC-stress sample (194):
no nesting; a deliberately nested call panics. Code identical to base (disassembly after masking
addresses), instruction counts identical on the 15 benches interp and AOT.

*Main-context root scan:* the GcLang binding is now the allocating context (`gc::Saruby = Ctx`).
`Ctx::alloc` / `alloc_with` hand their `&mut Ctx` to `Heap::alloc` as the language value, GC.start
/ `ObjectSpace.garbage_collect` / the fiber collector pass their `c`, and `vm::visit_roots` scans
that context through the borrow, skipping the by-place read of the main context and the
`FiberCtx::get` of a fiber when it is the one passed. For allocations through `Ctx` no collector
access overlaps the allocating code's `&mut Ctx` any more. The context-free `gc::alloc` /
`alloc_with` pass a static dummy `NO_CTX` and keep the old by-place scan: 85 call sites (53 in
functions that have `c`, 32 without; node.def literals, rt.rs AOT helpers, `alloc_class`, boot
inits incl. io.rs). Converting them retires `NO_CTX` (`unsafe` −2) and leaves the by-place read
only for an allocating fiber (main suspended in a switch). `unsafe` 380 -> 382, `static mut`
9 -> 10 (NO_CTX). A first form, `Saruby { ctx: *mut Ctx }`, cost gcchurn AOT +0.8 % (the struct's
store hoisted onto the fast path); as the context itself it is a register argument of the cold
call: within 1M of base on all 15 benches interp and AOT, startup in the same range.
optcarrot 82.97M -> 83.02M per frame (+0.06 %), checksum 59662. Checks (trial README): three builds 0
warnings at each commit, t/hand 0 differences, AOT vs `--plain` the same files as base (addresses),
panic scan and GC stress 0 panics, reuse / fork tests pass.

**Applied: NO_CTX** (trial README, section "NO_CTX"; base master `612ab2b` line, then master
`d8a2c40` merged). Every allocation now passes the allocating `&mut Ctx`: the 90 context-free call
sites (counted exactly by marking the allocators `#[deprecated]`) were converted in measured steps
(cold sites; `c` threaded into `alloc_class` / `new_module` / `str_to_inum` / the literal caches /
`defined?`; String literals; `new_object`; Envs and Procs; Hash and Float construction in node.def /
rt.rs; io.rs's boot sites), then `NO_CTX`, `is_no_ctx`, the context-free `gc::alloc` /
`alloc_with`, str_legacy.rs and the legacy Range / Float / Integer / Hash constructors removed.
`gc::alloc(c, ..)` is the only allocator; `vm::visit_roots` reads the main context by place only
while a fiber allocates (main suspended in a switch). `unsafe` 382 -> 380, `static mut` 10 -> 9.
Two codegen accidents found on the way and fixed: `Box<RHash>`'s drop glue left out of line in
`gc::drop_body` by an unrelated edit of `run_main` (bisected; the Hash arm now drops the buffers in
place), and `RHash::with_capacity` not inlined into `Ctx::hash_new_capa` (`#[inline]`). Cost against
master: equal on 10 of the 15 benches interp and AOT and on startup; optcarrot 83.02M per frame for
both; object AOT +0.62 % (register assignment around the inlined new_object in baked code;
`Object.new` itself is 1 instruction cheaper), nesteddata AOT +1.0 % (the context lives across the
Hash's two mallocs: one more callee-saved register in `hash_new_capa`, one argument move at the
call site), hash AOT +1.4 % (hash_set's register allocation; the old base had the same count, and
either of two unrelated cold edits flips it). Rows 41 to 43 of 2.4 were fixed alongside. Checks:
three builds 0 warnings, t/hand only the `cycle` fix differs, AOT vs `--plain` 0 after masking
addresses, panic scan and GC stress 0 panics, dev build 0 nested heap borrows, reuse / fork tests
pass.

## 8. Generated code

Trial: `2026-11-23-astrust-gen-unsafe` (classification, designs, measurements, patches). Base `15ad997`.
`ruby tools/gen_unsafe_catalog.rb [--root SAMPLE] [CODE_STORE...]` classifies every `unsafe` of the generated
sources and of baked crates by the astrustgen template that emits it (counted as in 1.1).

### 8.1 Where it came from

All of it traced back to a handful of templates of `lib/astrustgen.rb`: `NodePtr` is `*mut Node`, a node is a
`NodeOf<T>` behind it, and each per-node function cast and dereferenced it itself.

| template (saruby, base `15ad997`) | file | `unsafe` | what it does |
|---|---|---:|---|
| SPECIALIZE: operand struct, argument block, @spec hooks | node_specialize.rs | 221 | `&(*(n as *mut NodeOf<T>)).u`, raw field reads at bake time |
| emitted SD `call` / `test` / `spec` block, L_ text | node_specialize.rs | 120 | text: `unsafe { let np = ..; (*np).u.x, &mut (*np).u.cc, n.byte_offset(K) }` |
| emitted hole patch item (saruby hook) | node_specialize.rs | 64 | text: `asm!` store and `Pos::bound` of a bound line |
| HASH / PROF: operand struct, @bake hooks | node_hash.rs | 241 | `&(*(n as *mut NodeOf<T>)).u`, `(*np).u.tf.0` |
| CHILDREN: `&mut` operand struct, list elements | node_head.rs | 123 | `&mut (*(n as *mut NodeOf<T>)).u`; list slots written through the shared `&'static [NodePtr]` |
| ALLOC: head-hole store | node_head.rs | 32 | `(*n).head.line = line` after allocation |
| helpers: Dyn / DynList, kind table, flags, layout_tree, list_base | node_head.rs | 16 | head reads and writes through `NodePtr` |
| REPLACE: `&mut` operand struct, list element write | node_replace.rs | 123 | as CHILDREN |
| DISPATCH glue block | node_dispatch.rs | 105 | field reads, `&mut (*np).u.<ref>`, `&(*np).head.line` |
| DUMP | node_dump.rs | 105 | field reads |
| EVAL (node.def bodies, counted there) | node_eval.rs | 15 | |
| **total** | | **1165** | |

Baked crates (`all.rs` / `patch.rs`): fib 75 / 124, optcarrot 16990 / 22331. Per item: one per `T_` function
(call, test, spec; the block reads fields, takes `&mut` of @ref fields, the head line, children at
`n.byte_offset(K)`), in `L_` one per `nth` arm and `each` step plus the `unreachable_unchecked` arm, two per `H_`
hole item.

### 8.2 What replaced them

Node access goes through primitives in node_head.rs, each one `unsafe` with the contract stated once:
`node_u::<T>(n) -> &'a T` (the operand struct; T's kind checked by `debug_assert`), `node_u_mut` (CHILDREN on
layout_tree's copy, REPLACE), `node_kind` / `node_dispatcher` / `node_flags` / `node_set_flags` /
`node_hash_get` / `node_hash_set` (head fields one at a time, so no borrow of the head overlaps the dispatcher
write of patch binding), and `node_head_line` (a head hole). @ref operands were `NodeCell<T>` (`UnsafeCell`) with
`body_mut()` giving the body its `&mut T`; since 8.4 they are plain `T` with `Cell` fields and the body gets `&T`
(no primitive). Children in SDs are
`n.wrapping_byte_offset(K)` (an address, safe; the child's own `node_u` reads it). Head holes are set by
`node_allocate_with` before the node is shared. Lists in CHILDREN / REPLACE are copied, not written in place
(the base wrote through a pointer derived from a shared slice: undefined behaviour under Stacked / Tree Borrows;
fixed). Embedder hooks get `u` (the operand struct) in safe code; saruby_gen.rb / astrogre_gen.rb updated.

| | base | now |
|---|---:|---:|
| saruby generated sources | 1165 | 93 (node_head 13 primitives, node_specialize 64 hole-item text + 1 L_ text, node_eval 15 from node.def) |
| astrogre generated sources | 303 | 20 (12 primitives, 1 L_ text, 3 hash hooks of the embedder, node_free 1, node_eval 1) |
| calc generated sources | 102 | 15 |
| fib all.rs / patch.rs | 75 / 124 | 7 / 35 (L_ unreachable arms; H_ items 26) |
| optcarrot all.rs / patch.rs | 16990 / 22331 | 1274 / 3600 (L_ unreachable arms 1274 / 1402; H_ items 2198) |

Kept, for a measured cost: the `unreachable_unchecked` default arm of `L_::nth` (one per baked list). The safe
forms (`_ =>` the last child, or `unreachable!()`) add a range check where the index is not a constant (case
dispatch, loops LLVM does not unroll): optcarrot's main SD grew 377640 -> 399961 bytes (+402 `ja`, +393 `cmp`,
+72 `movslq`), all.so .text +1.04 %, +0.07 % instructions per frame against the kept form (base2 against base: 0.00 % / 0.02 % with SARUBY_PATCH=0).

Cost (instructions:u, 15 benches interp and AOT, base twice): every cell within 0.016 % except poly (-0.25 %
interp, -0.58 % AOT, faster); with `codegen-units=1` builds every cell within 0.008 %, poly included, and all 105
DISPATCH functions are identical machine code. The poly difference is `vm::call_slow` (hand-written), whose
register allocation differs between the two 16-unit builds (799 -> 785 instructions). fib's baked all.so is
identical machine code (addresses normalized). Startup (cgu1) +4.3 k instructions of 18.0 M (+0.02 %).
optcarrot (checksum 59662) per frame: 82.98 M against 83.02 M (base) and 83.02 M (base2); with SARUBY_PATCH=0 82.29 M against 82.25 M / 82.27 M.

### 8.3 What is left manual

The primitives are safe functions over a raw pointer, as the base's DISPATCH_* / hash_node already were: the
soundness argument is an invariant of the program, not of the types.

1. NodePtr validity: non-null, made by node_allocate / layout_tree, alive while a borrow is used (saruby never
   frees nodes; astrogre frees a pattern's nodes only after it ran).
2. Kind = struct: the generator emits `node_u::<x_struct>` only in code for kind x, and an SD only runs on a node
   of its hash; `debug_assert` checks it (a debug build over every 8th t/hand file, plain and AOT with the SD
   crates built with `-C debug-assertions=on`: 0 failures).
3. Operand fields other than the `Cell` state of @ref operands are not written while a borrow lives: only
   CHILDREN (on a copy not yet published) and REPLACE (no caller at run time in any sample) write.
4. (Closed by 8.4.) `NodeCell::body_mut` was not unique under re-entry: a recursive call site passed its
   `&mut CallCache` into `call_cached`, the callee reached the same node and made a second `&mut` while the first
   was a live (protected) argument. It was not only a model violation: LLVM used the `noalias` and an AOT run
   called the wrong method (8.4, bug table row 44).
5. SD child offsets: K is taken from the laid-out tree at bake time; valid because an SD is named by the tree's
   hash (its shape) and installed only on laid-out trees (`NF_LAID`).
6. `L_::nth`: i < len (EVAL_NTH! in bodies).
7. Patch binding copies and relocates code at run time; the hole items keep `asm!` and `Pos::bound`.

Designs considered and not built (the trial README has the arguments): a `NodeRef<'a, T>` handle made by an
`unsafe fn` per dispatch (the same obligations, one `unsafe` per dispatcher instead of none); `NodePtr` as
`&'static Node` / `NonNull` with an arena lifetime (makes 1 a type invariant; needs `Cell` head fields,
`Option` children, a lifetime parameter for astrogre's freed patterns through every signature, and changes to
every hand-written user of NodePtr); a checked swap for REPLACE (no run-time caller).

### 8.4 Inline caches under re-entry (closed)

Trial: `2026-11-23-saruby-ic-borrow` (base `b459991`, commit `f507c19`). A node's EVAL body held `&mut` to its own
@ref cache across calls; a re-entered dispatch of the same node made a second `&mut` while the first was live.
A detector build (each DISPATCH of a kind with @ref counts the node's live dispatches) found re-entry in 119 of
1896 t/hand files (`--plain`): node_call 76, node_call_blk 51, node_yield 25, node_call_x 21, node_opt_ltlt 5,
node_opt_eq 3, node_opt_aref 2, node_call_kw 2, node_send_sym 2, node_iasgn 1 (FrozenError's message runs
`#inspect`, which runs the same `@x = ..`). It was miscompiled, not only unsound: `call_spec_out` reused the
pre-call `cc.klass` after a re-entered call had refilled the cache, so an AOT site ran B#f for an A (bug table
row 44; correct with `-Zmutable-noalias=no`, which isolates the cause).

Fix: @ref operands are stored as `T` and the body receives `&T`; every cache keeps its state in `Cell` fields
(CallCache, YieldCache, ConstCache, IvarCache, KindMemo, SendGuard, LitCache, TyFb; OpCc and the polymorphic
table are `OnceCell<Box<..>>`). No `&mut` to a cache exists, so re-entry is sound by type, with no `unsafe` and no
invariant; `NodeCell` / `body_mut` are gone (generated sources 93 -> 91 `unsafe`, hand-written 380 -> 378: the
poly table's raw `Box` pointer). It is the semantics of ASTro's C version (a plain `struct method_cache *` the
compiler must assume a call may write), stated in the type. Option (a), ending the borrow before each call
with short `&mut` scopes, keeps an `UnsafeCell` primitive and a manual "no call in the scope" rule at every cache
API, so it was not built.

Cost (instructions:u, 15 benches interp and AOT, base twice, cgu16 and cgu1 builds): interpreter within
+0.13 / -0.85 %; AOT cells equal or faster (ivar -2.6 %: LLVM no longer speculates the `ic.idx` load that
`noalias` allowed; hash -1.4 / -2.7 %, iterators -1.7 %, gcchurn -1.6 %) except **block AOT +3.65 %** in both
builds: one stack reload per block step in one baked SD (`(1..N).each` with the block compiled in), a register
allocation of that function (the loop body has no cache access); four variants of the same loop (`times`,
`upto`, nested, two accumulators) are within 0.005 %. optcarrot (59662) per frame 82.91 M against 82.97 M /
82.97 M (base, base2); SARUBY_PATCH=0 82.37 M against 82.31 M / 82.30 M. Startup (cgu1) +16.5 k of 18.0 M.
Regression test: `tools/aot/ic_reentry_spec_klass_1123.rb` (run by `tools/aot_fork_test.sh`).
