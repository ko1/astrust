# `unsafe` in saruby: where it is necessary, what removing it cost, what removing it found

2026-11-22. The state described is master at **`f88d3e6`** ("saruby: todo.md (unsafe removal is
weighed against performance)"); every count below is of that tree unless a commit is named. Raw
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

## 1. Inventory of the remaining `unsafe`

### 1.1 Totals

Counted as `tools/heap_audit.rb` counts (line comments stripped; one per token, so `unsafe fn`,
`unsafe impl`, `#[unsafe(naked)]` and an `unsafe {}` block each count once).
`ruby tools/unsafe_catalog.rb` (added with this document) prints the per-category table of 1.2 and,
with `--list`, the category of every occurrence.

| where | `unsafe` | note |
|---|---:|---|
| `sample/saruby/src/**/*.rs` + `node.def`, hand-written | **365** | `heap_audit.rb` total; 52 files carry `#![forbid(unsafe_code)]` |
| of which compiled only with `FEATURES=raw_stack` | 5 | `src/stack_raw.rs`; the default build uses the checked `src/stack.rs` (0) |
| generated `src/node_*.rs` (from node.def by `lib/astrustgen.rb`) | 1131 | node_specialize 393, node_hash 235, node_head 163, node_replace 120, node_dispatch 102, node_dump 102, node_eval 16; `prism_ast.rs` 0 |
| generated per bake: the code store's SD crates | 259 for `fib.rb` | `all.rs` items 100 (83 files), `patch.rs` items 159 (119 files); grows with the program |
| `runtime/` (ASTrust runtime shared by the samples) | 49 | `astrust.rs` 5 (dlopen / dlsym), `astrust_loader.rs` 10 (memfd / mmap), `gc/` 34 |
| `lib/astrustgen.rb` (templates of the generated code) | 32 | |
| `sample/astrogre` (the regexp engine crate linked into saruby) | 265 lines | `cps.rs` 20, `main.rs` 1, the rest generated `node_*.rs` (lines containing `unsafe`, `grep -c`); not catalogued further here |

The number of `unsafe` tokens understates the trust surface in one specific way: **safe functions
with unsound signatures** are still in use outside the converted modules. At `f88d3e6`, ten
functions return an unbounded `&'static mut` from safe code (`heap_audit.rb` column
`fn->&'static mut`: object.rs 7 — `obj`, `rhash`, `rclass`, `rproc`, `rmethod`, `rbig` and the
macro-made `rary` family —, vm.rs 2 — `vm()`, `Ctx::frame()` —, gc.rs 1 — `heap()`), and
`heap_audit.rb` counts **449 call sites** of the old accessors (rt.rs 189, vm.rs 169, node.def 25,
object.rs 25, legacy.rs 14, str_legacy.rs 11, lib.rs 11, gc.rs 2, aot.rs 1, compile.rs 1,
inline_builtin.rs 1). Each of
those is an aliasing or lifetime assumption that no `unsafe` keyword marks. One reduction of the
count was of exactly this kind: the ASTro-principles work (378 -> 365) replaced
`unsafe { (*c.frame).line = line; }` in node.def by `c.frame().pos = line;`, i.e. it moved the
dereference behind the safe-signature `Ctx::frame() -> &'static mut Frame`
(`2026-11-21-saruby-astro-principles/README.md`, section "unsafe"). The count of 365 is therefore
an upper bound on *marked* trust, not on trust.

### 1.2 By category

`ruby tools/unsafe_catalog.rb` at `f88d3e6` (the classification inside vm.rs / rt.rs / node.def and
inside the heap files is by the text of each site, a heuristic; `--list` shows every decision).

| category | `unsafe` | files |
|---|---:|---|
| OS interface: libc wrappers | 67 | builtins/sys.rs |
| OS interface: global allocator | 13 | kalloc.rs |
| fiber: stacks and context switch | 23 | fiber_os.rs 22, fiber.rs 1 |
| code loading: code store, patch binding | 11 | aot.rs |
| parser FFI (prism) | 1 | prism.rs |
| node building: raw node reads | 6 (+1 in text emitted into generated code) | compile.rs |
| GC & heap base | 70 | heap.rs 34, value.rs 11, gc.rs 16, object.rs 7, lib.rs 2 |
| call path & frames | 158 | vm.rs 80, rt.rs 62, node.def 16 |
| value stack, raw backend | 5 | stack_raw.rs (not in the default build) |
| String buffer (SBuf) | 10 | sbuf.rs |
| **total** | **365** | |

Sub-categories of the two large groups:

| GC & heap base | `unsafe` | | call path & frames | `unsafe` |
|---|---:|---|---|---:|
| Value from a raw word (`from_raw`, `from_ptr`) | 15 | | frame chain (`c.frame`, `prev`, `FrameFull`) | 39 |
| entry / iseq / block / cref reads (leaked, H4) | 12 | | method entry (leaked `Box`) | 36 |
| static mut global (`VM`, `HEAP`, `REFINEMENTS_ON`, ...) | 8 | | block pointer (H4) | 17 |
| GC binding (gc.rs: `GcLang`, finalize, roots) | 8 | | iseq (never freed) | 16 |
| payload accessor (`body`, `body_mut`, `*_mut_and`, kind tests) | 8 | | cref chain (leaked) | 11 |
| legacy accessors (object.rs) | 7 | | static mut global | 10 |
| object header place access | 5 | | cache word reinterpreted (`transmute`, `as *const NewInfo`) | 10 |
| frame chain (H5) | 5 | | node pointer | 5 |
| context token (`Token::mint`) | 2 | | Value from a raw word (literal caches) | 5 |
| | | | GC root scan | 4 |
| | | | unchecked kind assumption (`unreachable_unchecked`) | 3 |
| | | | line position (`Pos`, patch holes) | 2 |

### 1.3 Per site group: operation, why it is not safe, alternative, cost

"Cost" is the cost of the safe (or checked) alternative. **M** = measured (trial cited),
**A** = argued or assumed, not measured, **—** = no safe alternative in Rust.

#### OS interface

| file:line | operation | why not safe Rust | safe alternative | cost of it |
|---|---|---|---|---|
| builtins/sys.rs (67 at f88d3e6, 73 with the blop layer; one per wrapper) | `read`, `write`, `pread`, `pwrite`, `open`, `close`, `dup`, `dup2`, `pipe2`, `poll`, `ppoll`, `epoll_create1`, `epoll_ctl`, `epoll_pwait2`, `eventfd`, `pidfd_open` (raw `syscall(434)`: glibc < 2.36 has no wrapper; the six are the blop layer's, io_design.md), `fcntl`, `flock`, `lseek`, `fsync`, `ftruncate`, `truncate`, `chmod`, `fchmod`, `chown`, `fchown`, `lchown`, `chdir`, `mkfifo`, `utimensat_cwd`, `umask`, `isatty`, `read_dir_raw` (3), `fd_metadata`, `waitpid`, `kill`, `fork`, `execvp`, `_exit`, `setsid`, `setpgid`, `getpgid`, `getsid`, `get*id` (6), `getrlimit`, `setrlimit`, `times`, `clock_gettime`, `clock_getres`, `nanosleep`, `set_itimer_real`, `signal`, `block_all_signals`, `setenv`, `unsetenv`, `clearenv`, `errno`, `set_errno`, `errmsg`, `local_tm` (2), libm `erf`, `erfc`, `cbrt`, `tgamma`, `lgamma_r` | FFI: a foreign function is `unsafe` to call. Each wrapper takes Rust types (slices, `&CStr`, `&mut`), so the pointer obligations are discharged by construction; one `SAFETY` line each (wave 4). Not dischargeable by a wrapper: `fork` (the child must exec at once or run alone), `setenv` / `unsetenv` / `clearenv` (race with `getenv` on other threads), `signal` (the handler's contract), `_exit`. | `std` covers part (`std::fs`, `std::process::Command`, `std::env::set_var`, which is itself `unsafe` since Rust 2024 for the same race); the rest needs a crate (`nix`, `rustix`) whose `unsafe` is the same FFI one level down. | A: moving the FFI into a crate changes where the `unsafe` is, not whether it exists. Not measured. |
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
| aot.rs:23, 30, 40, 69, 110 | `static mut STORE` (the loaded code store) | `static mut` | `OnceLock` / `Mutex` | A: one lock or one atomic load per first dispatch and per bake; off the steady-state path. Not measured. |
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
| heap.rs:125, 128, 366; gc.rs:36, 40, 51; aot.rs (above) | `static mut VM`, `HEAP`, `STACK_BASE`, `STRESS` behind `c.vm()`, `gc::heap()` | `static mut`; `vm()` has hundreds of context-free callers in rt.rs / vm.rs / node.def (the 449 legacy call sites) | `Vm` owned by `Ctx` (the stated end state, safe_heap.md "vm() and the method identities") | the check part measured in section 4 (`vmchk`); the ownership change not measured. |
| heap.rs:218–309 (12 + 5 frame) | method entries, crefs, iseqs, blocks and frames read through raw pointers, borrowing `c` | entries and crefs are leaked `Box`es, iseqs are never freed: never dangling, but stored as raw pointers because some are written in place (`me_set_visi`, `cref_set_refinements`) and because frames live on the native stack (H5) | `&'static` with `Cell` fields for the mutable parts (entries, crefs, iseqs); frames: no safe form (below) | A (entries): zero in principle; not built. |
| heap.rs:21; lib.rs:197; fiber_os.rs:175 | `Token::mint` (2 + 1) | the only way to make a `Ctx`; its contract (H2) is "contexts run one at a time" | — (an `unsafe` marker by design: it is what makes the rest checkable) | — |
| value.rs:18–21, 28, 30, 71, 76, 84, 86 (11) | `Value::from_raw` (NonZero unchecked), `from_ptr` | `Value` is `NonZeroU64`; `from_raw` is `unsafe` so that safe code cannot forge a heap `Value` (H1) | `NonZeroU64::new(..).unwrap()` / `.map(Value)` where the word is provably non-zero and not a heap address (fixnum, symbol, flonum, constants) | M: section 4 (`val`). |
| gc.rs:62, 71, 96, 102, 133–157, 183–186 | `from_ptr` of new objects, `unsafe impl GcLang`, `drop_body` / `finalize`, the conservative machine-stack scan | the collector frees memory and scans raw stack words | — | — |
| object.rs:119–143 (7) | the legacy accessors: `obj(v) -> &'static mut RObj`, `rhash`, `rproc`, `rmethod`, `rbig`, `rfloat_heap` with `unreachable_unchecked` on a wrong kind | unsound safe API kept for the 449 unconverted call sites | the context-borrowing accessors (done for every builtin) | M for the conversion: see section 2; the kind check is 2 instructions per access (pilot). |
| lib.rs:238 | `rt::BOOTED = true` | `static mut` | `AtomicBool` | M: section 4 (`atomic`). |

#### Call path and frames (vm.rs 80, rt.rs 62, node.def 16)

| group (count) | typical site | why not safe | safe alternative | cost |
|---|---|---|---|---|
| frame chain (39) | `(*c.frame).cref()`, `(*c.frame).blk`, `(*self.prev).cref()`, `&*(self as *const Frame as *const FrameFull)` (vm.rs:277), `(*g).prev` walks in rt.rs:500–519 | `Frame` records live on the native stack, one per Rust activation of a call, linked by raw `prev` pointers; `Ctx` holds the innermost. A reference with a lifetime cannot be stored in `Ctx` while the frame's owner (a Rust stack frame below) also uses it: the chain is self-referential through the machine stack. Lite frames are a prefix of `FrameFull`, distinguished by a flag. | frames in a `Vec<Frame>` owned by `Ctx`, linked by index (the value stack's `emu_idx` shape) | A: not built. The value stack's measurement of the same shape is the closest number: `emu_idx` AOT +16 to +89 % instructions (safe_stack.md, step 1), because the base and length are reloaded after every call. |
| method entry, leaked `Box` (36) | `&*me`, `(*me).kind`, `(*me).visi`, `(*me).owner` in `invoke*`, lookup, `respond_to`, `defined?` | `MethodEntry` is `Box::leak`ed and never freed, but kept as `*const` because `visi` is written in place (`Module#private`) | `&'static MethodEntry` with `visi: Cell<u8>` | A: no instruction change expected (a `Cell` read is a load). Not built. |
| block pointer, H4 (17) | `&*blk`, `(*blk).tag`, `(*blk).kind` | a `Block` lives in its creator's Rust frame or in a Proc; the callee receives `*const Block` and assumes it alive for the call (H4) | lifetimes on every builtin signature (`&'a Block`) | A: not attempted; it changes the `CFunc` type of every builtin. |
| iseq, never freed (16) | `&*is`, `(*is).bentry` | iseqs are leaked and immutable after build except `Cell` fields | `&'static Iseq` | A: zero; `c.iseq_ref` already returns `&'static Iseq` (heap.rs:271). |
| cref chain, leaked (11) | `(*cr).klass`, `(*cr).prev` loops (vm.rs:727, 917; rt.rs:350, 675) | leaked, `refinements` written in place | `&'static Cref` with a `Cell` | A: zero. |
| static mut global (10) | `vm()` (vm.rs:470), `REFINEMENTS_ON` (vm.rs:1589, 2253; rt.rs:180, 746), `BOOTED` (rt.rs:67, 107, 125) | `static mut` | atomics; `Vm` owned by `Ctx` | M: section 4 (`atomic`, `vmchk`). Precedent: REDEF as `AtomicU32`, identical instruction counts (`2026-11-21-saruby-loops-arith`, step 0007, commit c9c69a5). |
| cache word reinterpreted (10) | `transmute::<usize, CFunc>(cc.cfunc)` (vm.rs:1350, 1465), `&*(cc.fast as *const NewInfo)`, `&mut *(cc.poly as *mut [CallCache; POLY_WAYS])`, `yc.seen as *const Iseq` | call caches are plain words so that baked code can test them with one compare and the generator can hash them | typed fields: `Option<CFunc>` (niche, same size), `Option<&'static NewInfo>`, `Option<Box<[CallCache; N]>>` | M for `cfunc`: section 4 (`cfunc`). |
| node pointer (5) | `(*recv).head.kind`, dispatcher call `((*body).head.dispatcher)(c, body, fp)` | see "Generated code": nodes are a raw-pointer graph | — | — |
| Value from a raw word (5) | literal caches `Value::from_raw(lc.v)` (node.def:70, 101, 108, 115, 191) | the cache is a `u64` | `LitCache { v: Option<Value> }` (niche) | M: section 4 (`val`). |
| GC root scan (4) | vm.rs:2331–2383: `visit_ctx(&mut *vm.main_ctx)`, class fields walked as words, live stack as `&[usize]` | the collector writes slots through raw addresses | — | — |
| unchecked kind (3) | rt.rs:268, 274, 278 `Body::Env` else `unreachable_unchecked` | legacy shape | `unreachable!()` / the checked accessor | A: 1 compare-and-branch per env access in the interpreter; not measured. Precedent: the kind check of the safe heap accessors, 2 instructions per access, 0 to +2.1 % instructions (pilot). |
| line position (2) | vm.rs:230 `Pos::bound` (an `unsafe fn`), 235 the deref of an untagged `Pos` | `Pos` is a tagged word: an address of a line (interpreter) or the line itself (bound by the patch loader) | an enum | A: the 48-byte frame variant that stored the line itself was measured and rejected: +50–60M instructions on method_call / send (`2026-11-21-astrust-patch-binding`, "Frame layout"). |

**stack_raw.rs** (5, only with `FEATURES=raw_stack`): the unchecked backend of the value stack.
The default build uses `stack.rs` (`#![forbid(unsafe_code)]`); its cost is section 3.

#### Generated code

| where | count | what | why | alternative |
|---|---:|---|---|---|
| src/node_head.rs | 163 | `ALLOC_*` constructors and accessors: write fields of `NodeOf<T>` through `NodePtr` (`*mut NodeHead`), `(*n).head.line = line`, `node_laid`, `list_base` (`from_raw_parts` over child lists) | nodes are allocated once, laid out contiguously ("contiguous trees"), never freed, and rewritten in place: the dispatcher field is replaced at specialization and at the first dispatch of patch binding, and child slots are replaced by `node_replace`. Baked code addresses a node as `root + constant` and binds holes to node addresses, so the node's address is part of its identity. | an arena of nodes with `u32` indices and `Cell` fields: A, not built; it would change every `NodePtr` operand of every evaluator signature and the baked-code address arithmetic. |
| src/node_specialize.rs, node_hash.rs, node_dump.rs, node_replace.rs | 393, 235, 102, 120 | read node operands by kind to specialize, hash, dump, replace | as above | as above |
| src/node_dispatch.rs, node_eval.rs | 102, 16 | the dispatcher call through `(*n).head.dispatcher`; `match` over a child index with `unreachable_unchecked` | as above | the default arm as `unreachable!()`: A, not measured |
| code store SD crates (per bake; fib: 259) | | every `Disp::call` body is one `unsafe { .. }` (operand reads through `np`, `&mut (*np).u.cc`), `DispList::nth`'s `unreachable_unchecked`, `Value::from_raw(<literal>)`, and in `patch.rs` the hole items: `Pos::bound(&raw const ASTRUST_HOLE_line_N)` and the `asm!` store of a relocated immediate | the generator emits node reads as raw pointer reads; patch binding needs a relocation the compiler cannot express in safe Rust (`asm!` with a `sym` operand) | — for the holes; the rest as for node pointers |

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

Each line is a defect of the code *before* the conversion that the conversion either forced into
the open (a borrow error at a place that held a reference across a call) or that was found while
re-checking that place. Repro files are in the trial named. "now" = CRuby's output.

Attribution (to be completed per row): defects were found in three ways — (a) the safe accessors check the
object's kind where the legacy ones did not ("unchecked kind" rows: the SIGSEGVs), (b) the borrow rule
refused a reference held across a callback ("stale reference" rows: Ruby-level invalidation, not memory
safety Rust could name), (c) re-reading or testing (GC stress) the converted code. Only (a) and (b) are
effects of the type system; (c) is a by-product of the work. A per-row "found by" column is pending.

| wave | repro (one line) | before | kind |
|---|---|---|---|
| 2 | `h = (1..100).to_h { [_1, _1] }; n = 0; h.each { \|k, _\| h.delete(k); n += 1 }; p [n, h.size]` | `[59, 41]` (slots compacted under the iteration) | iterator invalidation |
| 2 | `class K; def initialize(h) = @h = h; def hash = 7; def eql?(o) = (@h.clear if $go; false); end; h = {}; 12.times { h[K.new(h)] = 1 }; $go = true; p h[K.new(h)]` | panic index out of bounds (probe kept the mask across `eql?`) | stale index across a callback |
| 2 | `o = Object.new; def o.hash = raise("boom"); a = [o]; 2.times { begin; a.hash; p :no; rescue => e; p e.message; end }` | `"boom"`, then `:no` (recursion guard left set) | state not restored on raise |
| 2 | `h = {"a" => 1}; begin; h.each { h.compare_by_identity }; rescue => e; p e.message; end; p h["a"]` | wrong message, `nil` | flag set before a failing rehash |
| 2 | `h = {}; h.replace({}.compare_by_identity); p h.compare_by_identity?` | `false` | |
| 2 | `S = Struct.new(:a); s = S.new(nil); s.a = s; p s.hash == s.hash` | process dies (unbounded recursion) | |
| String | `s = +"abc"; o = Object.new; o.define_singleton_method(:to_str) { s << "y" * 300; "c" }; s.chomp(o).size` | 3 (bytes of a moved buffer; CRuby 303) | **use of a stale `&'static [u8]` after the buffer moved** |
| String | `io.read(o)` with `o.to_int` doing `io.string = "x"` | `"lo w"` (old string) | stale reference across `to_int` |
| String | `Class.new(StringIO) { def initialize; end }.new.read` | **SIGSEGV** (nil read as a String) | unchecked kind |
| String | `s[o, 2] = "x"`, `s.byteslice(o, 5)`, `s.bytesplice(o, 5, "x")` with a `to_int` that clears `s` | panic (Rust index out of range) | length read before a conversion |
| String | `s.each_byte { s.clear }` | 150 iterations, zeros | iterating a buffer that is gone |
| String | `split` / `scan` / `gsub` / `scrub` with a block that changes the receiver | no "string modified" | |
| String | `sub`, `partition` argument / receiver order | read at the wrong moment | |
| 3 | `abort(1)` | **SIGSEGV** (a Fixnum read as a String) | unchecked kind |
| 3 | `autoload(1, "x")` | **SIGSEGV** | unchecked kind |
| 3 | `Module.new { refine(String) { import_methods 1 } }` | **SIGSEGV** | unchecked kind |
| 3 | `class String; def upcase = :x; end; p :y.upcase` | panic `:x is not a heap object` | |
| 3 | `"ab" =~ /(?<x>a)/; p Regexp.last_match(:x)` | `nil` (name read with `as_fix`) | unchecked kind |
| 3 | `SARUBY_GC_STRESS=1`: `p(/\A(?<x>.)(?<x>.)\k<x>\z/.match("abb").to_a)` | `["b", "b", "b"]` | **GC rooting: Strings held only in a Rust `Vec` during an allocation** (MatchData#to_a) |
| 4 | `r, w = IO.pipe; w.close; r.read(5, 123)` | **SIGSEGV** (an Integer read as a String) | unchecked kind |
| 4 | `File.realpath` / `Dir.pwd` on a name with a latin-1 byte | bytes replaced by U+FFFD | **undefined behaviour: `from_utf8_unchecked` on non-UTF-8 bytes** |
| 4 | `IO.readlines(path, chomp: true)` under `SARUBY_GC_STRESS=1` | TypeError (options Hash collected) | **GC rooting** |
| 4 | `marshal_dump` that clears its Hash and runs `GC.start` | dumps a freed String (79 bytes) | **use after free (GC)** |
| 4 | a `to_int` that closes the stream during `r.read(o)` | nil via a stale reference | stale reference across a callback |
| 4 | `IO.read(path, -1.5)` | "negative length -9007199254740989" (a Float read as a Fixnum) | unchecked kind |
| 4 | `marshal_dump` / `to_json` that adds a key to the Hash being dumped | dumped silently | iterator invalidation |
| 4 | 10 more compatibility differences fixed in the same files (`r.read(5, "x".freeze)`, `File.readlines(dir)`, `JSON.load` forms, `SystemCallError.new` forms, `SystemExit#message`, `RuntimeError#inspect`) | | found while re-reading each call |
| fiber | `a` resumes `b` resumes `a` | panic "entered unreachable code" (stack corrupted) | |
| method layer | a `Vm` field added after `main_obj` | left uninitialized (`MaybeUninit` + `ptr::write`), no diagnostic | **latent UB** removed by construction |

Found later (not by removing `unsafe`; the `unsafe` named is the one whose SAFETY claim was false):

| found | repro (one line) | before | unsafe / assumption | found by |
|---|---|---|---|---|
| 11-22 | AOT, patch binding on: `fork` a child, each side then first-dispatches new bodies (`tools/aot/fork_patch_arena_1122.rb`; ruby-bench knucleotide) | **SIGSEGV** / `memory allocation of 1.3e14 bytes failed` / a child killed (`SARUBY_PATCH=0`, `--plain` fine) | runtime/astrust_loader.rs `Arena::grow`: the RW window as `&'static mut [u8]`, "only this Arena uses it". The memfd is `MAP_SHARED`, so after `fork` the child's copy of the Arena (same `used`) wrote the parent's code pages: two processes held the one `&mut`. Fixed: an Arena records its pid; a child drops the inherited one unused and binds into a memfd of its own | backtrace (rsp truncated in patch-bound code outside every library), then a per-allocation pid trace: parent and child allocated the same addresses (`2026-11-22-saruby-knucleotide-segv`) |

Closed by construction and not reproduced (wave 3 and 4 READMEs): `clone_class` read the source
class through a `'static` alias while allocating; `Hash.ruby2_keywords_hash` iterated a `Vec` of
pairs across `hash_aset`; Rational / Complex / Time / Proc bodies held their Values only in a `Box`
during allocation; `thread::run_pass` iterated a Rust copy of the thread list while fibers ran;
`spawn_spec` kept pairs in a `Vec` across `to_str`; the JSON parser held a `&'static [u8]` into the
source String while running callbacks; the old global list of iterated hashes compared by address.

Performance defects the work uncovered (not bugs of behaviour): the "allocator accident"
(`__rust_alloc` inlined or not depending on module grouping under ThinLTO: nesteddata −12 % / +5 %
from moving functions between files; safe_heap.md, `ideas/rejected/saruby-legacy-shims-in-object-rs`),
and a store-forwarding stall from building a by-value payload on the stack and handing it to an
out-of-line allocator (strscan +30 % cycles on −3 % instructions; the same mechanism made gcchurn
460M -> 725M cycles at the pilot's merge on sp4, `2026-11-20-astrust-vs-astro/bisect_4a06252.txt`).

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

* **`val`, `atomic`, `cfunc`: removing 13, 10 and 2 `unsafe` costs no instructions.** Of the 44 cells,
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
| `static mut STORE` (aot.rs) as `OnceLock` | 5 | off the steady-state path | — |
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
2. **What the conversion found** (not all by the borrow checker itself; see the attribution note under table 2.4): 39 behavioural defects across the waves (table 2.4), of which
   5 SIGSEGVs, 1 undefined behaviour (`from_utf8_unchecked`), 3 GC rooting / use-after-free defects
   (MatchData#to_a with its Strings only in a Rust `Vec` during an allocation, IO.readlines' options
   Hash, marshal_dump + GC.start), 1 latent uninitialized field (boot's `MaybeUninit<Vm>`), and many stale-reference-across-callback defects (a `&'static [u8]` used after
   `to_str` moved the buffer; hash probes across `eql?`). Most were not memory-safety bugs Rust could
   name: they were *Ruby-level* invalidation bugs that the borrow rule exposed because it forbids
   holding the reference across the callback at all.
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
