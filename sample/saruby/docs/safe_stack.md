# The value stack without `unsafe`: design and what it costs

2026-11-20. Roadmap section 1, item 2. Raw data, scripts and the emulation patch:
`~/ruby/src/trials/2026-11-20-saruby-safe-stack/`.

## Answer

A value stack written entirely in safe Rust exists and passes everything the raw one passes
(`src/stack.rs`, `#![forbid(unsafe_code)]`). It is **not free**: in baked (AOT) code it executes
0 to 17 % more instructions per benchmark, the cost sitting in calls (fib +7.1 %, tak +17.1 %,
collatz +7.5 %; loops and array code +0 to 1 %), and 0 to 8 % more in the interpreter. That is
above the "about 3 %" bar, so it is a cargo feature, `safe_stack`, and the default build keeps raw
pointers. What changed for both builds is where the unsafe lives: the stack is now an API with
two backends, and node bodies and the runtime no longer contain any stack `unsafe`.

| | `unsafe` occurrences before | after |
|---|---|---|
| node.def | 50 | 22 (frame records and literal caches; none for the stack) |
| src/vm.rs | 101 | 68 |
| src/rt.rs | 81 | 62 |
| src/stack_raw.rs (default backend) | - | 6 (5 blocks, 1 in a comment) |
| src/stack.rs, src/slots.rs (`safe_stack`) | - | 0 code (`forbid(unsafe_code)`) |
| total, generated files excluded | 651 | 568 |
| generated (node_*.rs, prism_ast.rs) | 787 | 759 |

Cycle counts could not settle anything at the 3 % level: the machine was shared and loaded
(load average 6 to 40 during the runs), and two builds with identical instruction counts
differed by up to 20 % in minimum cycles. See "Measurements".

## What the stack was

Every evaluator was `fn(c: &mut Ctx, n: NodePtr, fp: *mut Value) -> RV`. Locals were
`*fp.add(i)`, `i` a literal in baked code. A call staged `[recv, -, args...]` at `c.sp` (a raw
pointer into one buffer) and the callee's frame started there, so the staged arguments were its
first locals. Only method entry compared against `c.stack_end`; staging itself was unchecked and
relied on 64 + 16 slots of slack. Blocks do not read their defining frame: captured variables
live in a heap Env (`fp[1]`), and a `Block` copies `self` and that env.

## Step 1: what a bounds check costs (emulation, before any redesign)

Two emulations were built on the raw-pointer code (patch in the trials directory):

* `emu_fat`: the frame is `(ptr, len)`, a slice by shape. Locals are checked against `len`;
  staging is checked once per call.
* `emu_idx`: the frame stays one word and each access goes through `c.<ptr>[index]`, checked
  against `c.<len>`: an index into a Vec owned by `Ctx`. (It pays a subtract and a shift per
  access that a true index would not, so it is an upper bound.)

Instructions are exact (repeat runs agree to 0.001 %); cycles are the minimum of 5 interleaved
runs. `e0` is the scaffolding compiled with both features off.

| AOT | emu_fat ins | emu_fat cyc | emu_idx ins | emu_idx cyc |
|---|---|---|---|---|
| fib | +11.7 % | +6.7 % | +35.1 % | +18.8 % |
| tak | +11.3 % | +1.7 % | +55.9 % | +19.7 % |
| method_call | +10.4 % | +10.8 % | +52.0 % | +33.2 % |
| while2 | +0.0 % | +0.3 % | +88.8 % | +85.8 % |
| nested_loop | +0.0 % | +6.4 % | +54.9 % | +48.9 % |
| collatz | +6.0 % | -0.4 % | +35.7 % | +16.9 % |
| sieve | +0.0 % | +1.4 % | +54.2 % | +36.8 % |
| aryidx | +1.6 % | -3.1 % | +15.6 % | +4.6 % |
| block | +6.4 % | -5.3 % | +16.7 % | +10.2 % |
| iterators | +6.3 % | +1.5 % | +16.8 % | +17.4 % |
| nbody | +4.4 % | +7.5 % | +26.1 % | +27.8 % |

| interpreter | emu_fat ins | emu_fat cyc | emu_idx ins | emu_idx cyc |
|---|---|---|---|---|
| fib | +11.1 % | +8.7 % | +15.9 % | +11.1 % |
| tak | +13.4 % | +8.2 % | +27.2 % | +16.1 % |
| method_call | +13.2 % | +8.4 % | +20.8 % | +16.2 % |
| while2 | +10.3 % | +4.3 % | +15.1 % | +5.6 % |
| nested_loop | +11.0 % | +5.7 % | +15.6 % | +9.0 % |
| collatz | +9.4 % | +3.3 % | +10.7 % | +7.1 % |
| sieve | +12.7 % | +3.7 % | +16.7 % | -0.0 % |
| aryidx | +8.2 % | +3.9 % | +11.1 % | +8.2 % |
| block | +8.8 % | +1.6 % | +12.3 % | +2.0 % |
| iterators | +8.8 % | +2.0 % | +12.4 % | +9.4 % |
| nbody | +10.6 % | +1.8 % | +14.5 % | +9.6 % |

Cycle spread (max-min)/min in that run was 1 to 80 % per cell, so read the cycle columns as
"same sign as instructions, about half the size", no more. `e0` matched the baseline's
instruction counts except while2 AOT (907 M against 947 M: exactly one instruction per loop
iteration, from an unrelated code-layout change). That is the size of effect that is not signal.

What the generated code showed (objdump of `code_store/all.so` for fib):

* In `emu_fat`, the whole `.so` has **one** remaining out-of-bounds branch. Once method entry has
  checked `len` against its frame size, LLVM deletes every check on a literal index: `len` is an
  SSA value, so the fact survives calls. Loops cost nothing (while2, sieve, nested_loop +0.0 %).
* What `emu_fat` pays for is the length itself. In baked code that is per call: deriving the
  callee's `(ptr, len)`, two compares, one more callee-saved register. In the interpreter it is
  per node: every dispatcher takes and forwards a fourth argument.
* In `emu_idx` nothing folds. The pointer and length are reloaded from `Ctx` after every call,
  because the callee may have changed them, and the compare stays.

So: a constant-index check is free if the length is at hand, and the price is in how the length
travels. That decided the design.

## Design

One API, two backends, chosen at build time. Nothing outside the backends knows which is in use.

```
slots.rs      Slot = Cell<Value>, FRAME = 2^16, stack memory (leaked, recycled)   forbid(unsafe_code)
stack.rs      checked backend, feature safe_stack                               forbid(unsafe_code)
stack_raw.rs  unchecked backend, default                                        all the stack's unsafe
```

API: `Fp` (a frame, as evaluators get it), `Stk` (a position: a staged call, a frame base),
`fp[0]`, `slot(fp, i)`, `st[i]`, `stage(c, n)`, `frame(c, st)`, `at_entry`, `enter`, `head`,
`put`, `nil_fill`, `with_args`, `c.set_sp / top / live / has_room / sp_mark / sp_reset`,
`stack_new / stack_release`.

### Checked backend

**Slots are `Cell<Value>`.** This is what removes the need for unsafe. Frames overlap by design
(staged arguments are the callee's parameters), a `Frame` record points at its frame's
self and env slots, and the GC reads every live slot while evaluators hold their frames. With
`&mut [Value]` each of those is a second reference to memory someone holds exclusively. With
shared references to Cells they are ordinary safe code, and LLVM is told the truth: a slot may
change across a call. That is also what a moving collector will need.

**A stack is `&'static [Slot]`**, leaked. A finished fiber's stack goes to a pool and is reused,
never freed, so `'static` is true and frames can be stored anywhere without lifetimes.  Large
stacks are mapped from a file of nils (`fiber_os::nil_slots`), so untouched pages cost nothing and
a released stack's pages can be dropped (they read as nil again: still valid Values).

**`Stk = &'static [Slot]`** is a position: the slots from there to the end of the stack.

**`Fp = &'static [Slot; FRAME]`**, a reference to a fixed-size array, is what evaluators receive:
`fn(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV`. It is one word, so the dispatcher signature and
the interpreter are as before. In baked code a slot index is a literal and is checked by rustc.
In the interpreter `slot(fp, i)` indexes with `i as u16`, which is below 2^16 by type: no check
and no extra instruction. `compile.rs` keeps every frame within FRAME: a scope's locals beyond
32768 go to its heap env, and more than 65536 slots is a compile error.

**sp is `c.sp`, the number of free slots** (the distance from the end), next to `c.stk`, the
whole stack. `stage(c, n)` checks `room >= n + FRAME` and `room <= stk.len()`, returns
`&stk[len - room..]` and lowers the count. `n + FRAME` free slots guarantee that a frame fits at
the staged position, so `frame(c, st)` (`first_chunk::<FRAME>()`) is the stack overflow check
and folds away after a `stage`. A stack's last FRAME slots can therefore not start a frame
(0.8 % of the main stack; fiber stacks were enlarged by FRAME).

**A call**

```rust
let base = stage(c, 2 + k)?;                 // [recv, -, args...] at sp
base[0].set(EVAL_ARG!(c, recv)?);
for i in 0..k { let v = EVAL_NTH!(c, args, i)?; base[2 + i].set(v); }
...
let callee_fp = frame(c, base)?;             // no code: stage() proved it
let r = EVAL_SPEC!(c, callee, cc.fast as NodePtr, callee_fp);
c.set_sp(base, 0);
```

and method entry does `enter(c, fp, nlocals, nparams)` (sp goes from above the staged arguments
to above its locals) and nil-fills.  A caller that did not just stage sets sp first with
`at_entry(c, base, argc)` (2026-11-21; before, every call reset sp to the base and the entry
lowered it by `nlocals`: one more store per call). The zero-copy
overlap needs nothing special: `base` and the callee's `fp` are two shared views of the same Cells.

**cfuncs** take `&[Value]`. A `&[Cell<Value>]` cannot be turned into one in safe code, and
rightly: the type would promise that nothing writes those slots. `with_args` copies the staged
arguments to the native stack (nothing for 0, a fixed 2 or 6 slots, a Vec beyond). The originals
stay staged below sp, so the copies are rooted.

**Fibers** each own a `Ctx` with its own stack from `stack_new`, returned to the pool when the
fiber ends or its Fiber is collected (`FiberData`'s `Drop`).

**GC roots**: `c.live()` is `&stk[..len - room]`; `mark_roots` and `mark_fibers` iterate it.

### Why it is sound

There is no unsafe in `stack.rs` or `slots.rs`, so memory safety of the stack does not rest on an
argument: every access is an index into a slice or array, checked by rustc or at run time.
In particular `c.sp` is an ordinary number. A wrong value gives a wrong slot, a
SystemStackError or a panic, never an access outside the stack.

What a wrong sp can still break is **liveness**, exactly as before: `live()` is the GC's view,
and a live value above sp would not be scanned. That is the rooting discipline of the heap
(roadmap item 1), not of the stack. The call protocol keeps it absolute where it matters: the
caller sets sp back from `base` after every call, so an unbalanced path cannot accumulate. (A
baked call enters with the sp its own `stage` left; an imbalance in its argument evaluation
could only leave sp lower, which keeps more slots live, never fewer.)

A recycled stack is not cleared. Slots above sp are dead by definition and no evaluator reads a
slot it has not written; `with_args` may copy such a slot but never exposes it.

### Unchecked backend (default)

`Stk` and `Fp` are one raw pointer, `c.sp` is a pointer, indexing is unchecked, `stage` is a load
and a store, `enter` is the old `stack_end` compare, `with_args` lends the staged slots
themselves. It reproduces the old code: against the commit before this work, AOT instruction
counts are within 1 % on seven benchmarks and 1.5 to 3.4 % lower on the other four (table below).

Its functions are not marked `unsafe` but are only correct under four rules, listed at the top
of the file: indices within the frame or staged count (R1), handles used only while their frame
is live (R2), sp above everything live (R3), nothing writes arguments lent to a cfunc (R4).
Those are the rules the old code followed implicitly in 100 places; now they are stated once and
there is a way to test R1: **the checked backend turns any out-of-range index into a panic or a
SystemStackError**, so running the differential tests with `FEATURES=safe_stack` audits the
default build for it. R2 to R4 stay a discipline (the checked backend makes them harmless to
memory, it does not detect them).

### `unsafe` that remains around frames, and why

* `src/stack_raw.rs`: 5 blocks, described above.
* `(*c.frame).line`, `.blk`, `.kind`, `.id`, `.cref()` in node.def (15) and vm.rs/rt.rs: the
  chain of `Frame` records on the native stack linked by raw `prev` pointers. That is the frame
  chain, a separate surface; the value stack no longer contributes to it (`Frame.fp` is a safe
  `&[Slot; 2]` in the checked backend).
* `vm.main_ctx` (was `static mut MAIN_CTX`) in `visit_roots` and the fiber list: how the GC finds contexts.
* `Value::from_raw` on literal caches, `&*fr.iseq()`: unrelated to the stack.

## Alternatives, with their cost

| design | status | cost |
|---|---|---|
| index into a Vec owned by `Ctx` (`c.stack[base + i]`) | rejected, measured (`emu_idx`) | AOT +16 to +89 % instructions, +5 to +86 % cycles: pointer and length reload after every call |
| fat slice `fp: &[Slot]` in the dispatcher signature | rejected, built and measured | AOT fib +8.4 %, block +13.6 %, iterators +13.5 % instructions; interpreter +10 to +16 % (fib +13.5 %, method_call +15.7 %): a fourth dispatcher argument on every node |
| `&mut [Value]` with `split_at_mut` for the callee | rejected by argument | the GC scan and `Frame.fp` would read memory a live `&mut` covers; `noalias` lets LLVM delay or drop stores to slots, which loses roots for a precise collector, and a moving one would write under it |
| **`&[Cell<Value>; FRAME]` window + position slices from `Ctx`** | chosen for the checked backend | table below |
| the same with cfunc arguments passed zero-copy through one `unsafe` cast | rejected, measured | saves at most 2.4 % instructions (mathfn AOT), within 0.25 % on most; not worth an unsound cast |
| static stage offsets per call site (a frame of fixed size, no sp traffic) | not done | would remove the staging cost of cfunc calls (collatz) but not the callee-frame check of Ruby calls; needs a pass in compile.rs |

The irreducible part: a callee's frame is a pointer the caller computes, and safe Rust wants a
proof that it is in bounds. The raw code gets it from one compare against `stack_end` at method
entry and derives everything else from `fp` (in baked code the stage position is a constant
offset from `fp`). The checked code has to obtain the position from something that carries a
length, which is `c.stk`: three loads and two compares per staged call, reloaded after every
call. About 8 instructions per Ruby call.

## Measurements

`work/bench.sh`: variants interleaved inside each repetition, `perf stat -e cycles:u,instructions:u`,
5 repetitions, minimum; spread = (max-min)/min. Shared machine, load average 6 to 17 during this run.
`base` = commit 2ae4b98; `raw` = this branch, default; `safe` = this branch, `FEATURES=safe_stack`.
Millions.

| AOT | base ins | raw ins | safe ins | base cyc (spread) | raw cyc (spread) | safe cyc (spread) |
|---|---|---|---|---|---|---|
| fib | 2694 | 2696 (+0.1 %) | 2887 (+7.1 %) | 663 (1.9 %) | 653 (2.4 %) -1.6 % | 698 (2.3 %) +5.2 % |
| tak | 5752 | 5754 (+0.0 %) | 6738 (+17.1 %) | 1543 (62.1 %) | 1517 (62.5 %) -1.7 % | 1731 (74.5 %) +12.2 % |
| method_call | 4327 | 4179 (-3.4 %) | 4329 (+0.0 %) | 932 (88.9 %) | 902 (82.5 %) -3.1 % | 993 (78.5 %) +6.6 % |
| while2 | 947 | 949 (+0.2 %) | 948 (+0.1 %) | 346 (2.7 %) | 418 (7.6 %) +20.8 % | 423 (2.2 %) +22.2 % |
| nested_loop | 3076 | 3077 (+0.1 %) | 3077 (+0.0 %) | 596 (2.2 %) | 596 (5.0 %) -0.1 % | 636 (4.3 %) +6.6 % |
| collatz | 2876 | 2878 (+0.1 %) | 3092 (+7.5 %) | 726 (1.8 %) | 728 (2.7 %) +0.2 % | 782 (2.5 %) +7.8 % |
| sieve | 959 | 961 (+0.2 %) | 970 (+1.1 %) | 387 (5.1 %) | 375 (9.0 %) -2.9 % | 412 (1.9 %) +6.6 % |
| aryidx | 77 | 78 (+0.8 %) | 78 (+1.4 %) | 64 (5.9 %) | 61 (7.4 %) -4.6 % | 62 (6.5 %) -3.0 % |
| block | 1887 | 1859 (-1.5 %) | 1903 (+0.9 %) | 827 (3.3 %) | 806 (5.0 %) -2.4 % | 798 (9.4 %) -3.5 % |
| iterators | 5100 | 5019 (-1.6 %) | 5154 (+1.0 %) | 2114 (6.5 %) | 2184 (4.6 %) +3.3 % | 2178 (8.7 %) +3.0 % |
| nbody | 3072 | 3021 (-1.6 %) | 3148 (+2.5 %) | 947 (1.6 %) | 936 (4.7 %) -1.2 % | 1003 (1.2 %) +5.9 % |

| interpreter | base ins | raw ins | safe ins | base cyc (spread) | raw cyc (spread) | safe cyc (spread) |
|---|---|---|---|---|---|---|
| fib | 6666 | 6668 (+0.0 %) | 7095 (+6.4 %) | 1772 (2.4 %) | 1832 (8.0 %) +3.4 % | 1896 (4.4 %) +7.0 % |
| tak | 13159 | 13161 (+0.0 %) | 14214 (+8.0 %) | 3823 (63.2 %) | 4240 (68.3 %) +10.9 % | 4097 (67.5 %) +7.2 % |
| method_call | 13677 | 13678 (+0.0 %) | 14628 (+7.0 %) | 6197 (4.3 %) | 6762 (3.7 %) +9.1 % | 6635 (4.3 %) +7.1 % |
| while2 | 5027 | 5028 (+0.0 %) | 5028 (+0.0 %) | 1610 (61.7 %) | 1588 (63.3 %) -1.3 % | 1691 (49.8 %) +5.0 % |
| nested_loop | 11321 | 11323 (+0.0 %) | 11322 (+0.0 %) | 4935 (18.9 %) | 3485 (64.3 %) -29.4 % | 3560 (56.0 %) -27.9 % |
| collatz | 9193 | 9194 (+0.0 %) | 9624 (+4.7 %) | 2760 (4.6 %) | 2872 (3.9 %) +4.0 % | 2813 (6.8 %) +1.9 % |
| sieve | 3557 | 3558 (+0.0 %) | 3558 (+0.0 %) | 1659 (2.1 %) | 1663 (2.0 %) +0.3 % | 1654 (1.9 %) -0.3 % |
| aryidx | 140 | 141 (+0.9 %) | 141 (+1.1 %) | 95 (4.1 %) | 91 (10.6 %) -4.0 % | 93 (6.2 %) -1.7 % |
| block | 2561 | 2533 (-1.1 %) | 2578 (+0.6 %) | 1161 (8.1 %) | 1150 (8.5 %) -0.9 % | 1179 (4.0 %) +1.6 % |
| iterators | 6981 | 6901 (-1.2 %) | 7052 (+1.0 %) | 3086 (1.4 %) | 3205 (3.9 %) +3.3 % | 3217 (2.9 %) +4.3 % |
| nbody | 5536 | 5543 (+0.1 %) | 5773 (+4.3 %) | 1553 (3.1 %) | 1594 (4.4 %) +2.7 % | 1623 (6.6 %) +4.6 % |

How far to trust the cycles: `raw` executes the same instructions as `base` to 0.2 % on most
rows, yet its minimum cycles differ by -29 % to +21 %. That is the noise of this machine in this
session, and it is larger than most of the `safe` deltas. On the AOT rows whose spread stayed
under 5 % for all three builds, `safe` is +5.2 % (fib), +7.8 % (collatz) and +5.9 % (nbody),
close to or above its instruction overhead; but nested_loop shows +6.6 % with identical
instructions, which cannot be told from noise here. So the cycle columns agree in sign with the
instruction counts on call-heavy code and settle nothing finer. A quiet machine (sp4) is needed.

Other checks: the checked backend's cfunc argument copy against a zero-copy cast (an earlier
iteration, before the no-copy path for 0 arguments), AOT instructions: mathfn 2436 M against
2378 M, collatz 3177 against 3114, strops 2358 against 2343, ary 1892 against 1916; eight of the
eleven benchmarks above within 0.25 %.

## Compatibility

* Output comparison over rubyharness `t/hand` (1699 files; stdout+stderr and exit status against
  the starting commit, 60 s limit), the same result for the default build and for `safe_stack`:
  1670 identical, 14 time out in both, 15 print addresses, pids or random numbers and are
  identical once numbers are blanked. No file differs.
* `run_specs.rb --jobs 4 --timeout 15 ... --dir t/hand`, TOTAL row (PASS FAIL CRASH WCRASH TIMEO REFTO):

  | build | run | row |
  |---|---|---|
  | starting commit | first (1698 files) | 23058 10346 0 4 17 0 |
  | starting commit | second | 23063 10383 0 6 15 0 |
  | starting commit | third | 23044 10405 0 5 16 0 |
  | default (raw) | first | 23044 10405 0 4 17 0 |
  | default (raw) | second | 23044 10405 0 6 15 0 |
  | `safe_stack` | first | 23044 10405 0 6 15 0 |

  The test directory was being edited by other sessions during the day and slow tests sit at
  the 15 s limit under load, so the starting commit's own row moved between runs; its latest
  run and both new builds agree on PASS and FAIL.
* Each of the eleven benchmarks prints what `ruby` prints, interpreter and AOT, in both builds.
* `make` in sample/saruby (default and `FEATURES=safe_stack`) and in sample/calc: no warnings.

## Using it

```sh
make                        # default: stack_raw.rs
make FEATURES=safe_stack    # checked: stack.rs (code stores are per build id; rebake)
```

To make the checked stack the default, add `default = ["safe_stack"]` to `[features]`.

## Possible follow-ups

* Static stage offsets (table above), to bring cfunc-heavy code to parity under `safe_stack`.
* cfuncs taking `&[Slot]`, which removes the argument copy and is what a moving GC needs anyway.
* A quiet-machine run of the cycle columns.
