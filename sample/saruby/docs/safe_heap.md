# Heap objects without `&'static mut`: design, pilot, migration recipe

2026-11-21. Roadmap section 1, items 1 and 3. Raw data, scripts and the emulation patches:
`~/ruby/src/trials/2026-11-21-saruby-safe-heap/`.

## Answer

* **Chosen: payload references borrow the context** (design (a)). `c.ary(v) -> &[Value]` borrows
  `c` shared, `c.ary_mut(v) -> &mut Vec<Value>` borrows it exclusively, and everything that can
  allocate, collect or run Ruby code takes `&mut Ctx`. "Two `&mut` to one body" and "a body
  reference held across a GC point" are then the same compile error (E0499 / E0502). No run-time
  state is added.
* What it does cost at run time is unrelated to borrowing: a safe accessor must **check** that
  the `Value` is a heap object of the right kind, where `rary` assumed it
  (`unreachable_unchecked`). Measured on the Array pilot: 0 to +2.1 % instructions per benchmark (two
  instructions per checked access; `ary` AOT is the +2.1 %), and no difference in cycles that
  can be told from code placement (-3.9 % to +1.6 % in baked code; the interpreter moves by
  more than that between two builds that execute the same instructions).
* The pilot: `src/array.rs` and `src/builtins/array_methods.rs` are `#![forbid(unsafe_code)]`,
  name none of the old accessors (a compile error if they do), and no `&'static mut` to an
  array body leaves them. The four hottest array nodes of `node.def` use the same accessors.
* All of the `unsafe` of heap access is in `src/heap.rs` (12 occurrences); two
  functions carry the argument (`body`, `body_mut`).
* `static mut` method identities (`BO_EQ_ME`, `BO_INIT_ME`, `C_NEW_ME`, `M_EQQ_ME`,
  `KERNEL_HASH_ME`, `R_EACH_ME`) and `MAIN_CTX` are gone: they are fields of `Vm`.
  `static mut VM` itself stays until the last `vm()` caller is converted (634 call sites).
* Not done, by design of a pilot: 2302 call sites of the old accessors in 50 files (table at the
  end), and the legacy allocators that do not take the context (see "What is still open").

## The problem

```rust
pub fn rary(v: Value) -> &'static mut Vec<Value> { ... unsafe { &mut *v.ptr() } ... }   // object.rs
```

A safe function that returns an unbounded `&'static mut`:

1. two calls give two live `&mut` to the same `Vec` (undefined behaviour by itself, and LLVM is
   told `noalias`);
2. the reference can be held across `ary_push` (the buffer moves), across a block call (the
   block can do the same), and across an allocation (the collector reads the body, a moving one
   would rewrite it, and if the reference is all that is left of the object the sweep frees it);
3. the body kind is assumed, not checked: `rary(a_string)` is `unreachable_unchecked`.

There were 2502 call sites of such accessors (`obj`, `rary`, `rstr`, `rhash`, `rclass`, `rproc`,
`rmethod`, `rbig`, `rdata`, `ivars_of`, `str_bytes`, `vm`, and the context-free `ary_*`
functions built on them), plus 33 module-local functions returning `&'static mut`
(`fn th(v) -> &'static mut RThread`, ...). `design.md` asks the programmer not to hold them
across calls; nothing checked it.

## Design

### An object has three parts with three rules

```
RObj { klass: Value, flags: u32, gc: GcHeader,     header: read and written BY VALUE, no token
       body: Body }                                 tag (1 byte): read by value, never written after alloc
                                                    payload: BORROWED from the context
```

* **Header** (`klass`, `flags`, the kind of `body`): `obj_klass(v)`, `obj_set_klass(v, k)`,
  `obj_flags(v)`, `obj_set_flags(v, bits)`, `obj_clear_flags(v, bits)`, `is_array(v)`,
  `is_string(v)`, ..., `is_frozen(v)`. Plain functions of a `Value`. They never create a
  reference to anything in the object (the accesses are place reads and writes through the raw
  pointer), which is `Cell` semantics without changing the struct: they cannot conflict with
  any reference, so they need no token and can be called while a payload is borrowed.
  `Body` is now `#[repr(u8)]`, which fixes the tag at offset 0 of the body and the payload
  after it, so that "reads the tag only" is a statement about a defined layout.
* **Payload** (the `Vec<Value>` of an Array, `RString`, `RHash`, `ClassData`, ...): only through
  methods of `Ctx`, whose result borrows the context:

  | read (`&self`) | write (`&mut self`) | payload |
  |---|---|---|
  | `c.ary(v) -> &[Value]` (`c.ary_vec` for the `&Vec`) | `c.ary_mut(v) -> &mut Vec<Value>` | Array |
  | `c.rstring(v) -> &RString`, `c.str_bytes(v) -> &[u8]` | `c.rstring_mut(v)` | String |
  | `c.hash(v) -> &RHash` | `c.hash_mut(v)` | Hash |
  | `c.class(v) -> &ClassData` | `c.class_mut(v)` | Class / Module |
  | `c.ivars(v) -> &IvBuf` | `c.ivars_mut(v)` | plain object |
  | `c.proc_data(v)`, `c.method_obj(v)`, `c.big(v)` | `..._mut` | Proc, Method, bignum |
  | `c.data::<T>(v) -> Option<&T>` | `c.data_mut::<T>(v)` | native payloads (`RData`) |

  Each checks `v.is_heap()` and the body kind and panics otherwise (cold, out of line).
* **Allocation** takes the token: `c.alloc(klass, body)`, and on top of it `c.ary_new()`,
  `c.ary_new_from(&[..])`, `c.ary_from_vec(vec)`, `c.ary_push(a, x)`, `c.str_new(b)`, ...

### The token is the context

Every builtin already has `c: &mut Ctx`, and everything that can run Ruby code or switch fibers
already takes it (`funcall`, `yield1`, `err`, `num_to_long`, ...). So the borrow checker's rule
for `c` is exactly the rule wanted for the heap:

```rust
let a = c.ary(v);               // &[Value], borrows c
let x = c.ary_new();            // error[E0502]: cannot borrow `*c` as mutable because it is also borrowed as immutable
a[0]
```

and in the other direction nothing is lost: two shared borrows may overlap
(`c.str_bytes(a).cmp(c.str_bytes(b))`), a class value can be read inside a call that takes `c`
(`err(c, c.vm().c_index_error, ..)` compiles: two-phase borrows), and a borrow ends at its last
use, so "read, call out, read again" needs no scopes.

A `Ctx` cannot be made by safe code: it has a field `tok: heap::Token`, and `Token::mint()` is
`unsafe` (two call sites: the main context in `lib.rs`, a fiber's in `fiber.rs`). A third one,
a blank context `rbconfig_init` built while the real one was live, was removed.

### Two objects at once

* Reading two: just do it, `&Ctx` is shared.
* Writing one while reading others: `c.ary_mut_and(dst) -> (&mut Vec<Value>, Others<'_>)`.
  `Others` reads any object **except** `dst`: `others.ary(src) -> Option<&[Value]>` is `None`
  when `src == dst` (the caller then uses `extend_from_within` or the like). Sound because
  distinct objects own disjoint payloads. `c.ary_concat(dst, src)` in array.rs is the example.
* Writing two: not needed in the pilot; the same construction with two exceptions when it is.

### `vm()` and the method identities

`vm() -> &'static mut Vm` has the same defect as `rary`. The same cure: `c.vm() -> &Vm` and
`c.vm_mut() -> &mut Vm` borrow the context. Builtin class values are `Copy`, so
`c.vm().c_array` never holds the borrow.

The six method identities and `MAIN_CTX` were `static mut` raw pointers read with `unsafe` at
each use. They are now `Vm.ids: MethodIds` and `Vm.main_ctx`, written once by boot /
`run_main`, read as `vm().ids.c_new` (old code) or `c.vm().ids.c_new` (converted code) with no
`unsafe` at the use (comparing raw pointers is safe). The root scan still dereferences
`vm.main_ctx` in one `unsafe` block.

End state for the globals, not reached here: `Ctx` holds the pointer to `Vm`, `Vm` owns the
`Heap`, and the collector is entered with the context (`c.alloc`), so that `VM` and `HEAP` stop
being statics at all. What blocks it: `vm()` has 634 callers, and the GC binding (`gc.rs`
`Saruby`, stateless) finds its roots through the globals. The 48 remaining `static mut` are
counted per file below; those holding Values (`thread.rs`, `fiber.rs`, `process.rs`, ...)
become `Vm` fields visited by `visit_roots`.

### The `unsafe`, and why it is sound

`src/heap.rs`. Two functions do the work:

```rust
fn body(_c: &Ctx, v: Value) -> &Body { unsafe { &(*robj(v)).body } }
fn body_mut(_c: &mut Ctx, v: Value) -> &mut Body { unsafe { &mut (*robj(v)).body } }
```

(`robj` panics unless `v.is_heap()`.) The typed accessors are safe matches on their result.
The rest: five header accessors and the kind tests (raw place reads / writes), `ary_mut_and`
(one more `&mut` under a shared borrow, justified by `Others`), `vm` / `vm_mut`, `Token::mint`.

Claim: under H1 to H3 below, a payload reference is never used after its payload was freed or
moved, and no `&mut` to a payload coexists with another reference to it.

* **H2 gives exclusivity.** Every payload reference is created by `body` / `body_mut` with the
  lifetime of a borrow of a `Ctx`. All `Ctx` values come from `Token::mint`, whose contract is
  that contexts run one at a time: a fiber's context is used only between switch-in and
  switch-out, and the context that resumed it is, for that time, mutably borrowed by the
  `resume` call on its stack. So at any moment every usable `&Ctx` / `&mut Ctx` is a reborrow
  of one context, and Rust's rules for that one value give "many readers or one writer" for the
  whole heap. `Others` is the only refinement: one writer plus readers of other objects.
* **Nothing frees or moves a payload behind a borrow.** A payload changes through a `&mut`
  accessor (needs `&mut Ctx`), or in the collector's finalizer, which runs only inside an
  allocation (needs `&mut Ctx`). Marking, which makes its own `&mut RObj`, is also only inside
  an allocation.
* **H3 keeps the header out of it.** Header accessors make no reference, and payload references
  do not cover the header or the tag byte, so a header read during a payload borrow touches
  disjoint bytes.
* **The kind is checked**, so a wrong `Value` is a panic. The tag of a live object is written
  only by `gc::alloc`; the two places that assign a whole body (`Range#initialize`,
  `Marshal.load` of a Range) keep the kind.

### What Rust cannot check (assumptions)

* **H1, liveness.** `c.ary(v)` dereferences `v`. That `v` still names the object it was made
  for is the rooting discipline, as before: a `Value` in a Rust local is found by the
  conservative machine-stack scan, a `Value` that lives only in a Rust heap buffer
  (`Vec<Value>`, a `Box`) across a GC point is not. The borrow rule removes one way to lose a
  root that the scan could not see: a `&[Value]` into the body kept after the `Value` itself was
  dead (the reference points into the malloc'ed buffer, not into the GC slot, so the scan does
  not recognize it). It does not root anything by itself.
  `Value::from_ptr` is now `unsafe` (three callers, all in `gc.rs`), so safe code cannot forge
  a heap `Value`. A stale one is not provably harmless: with the mark-sweep backend a freed
  slot keeps a `Body::Free` tag and the kind check turns a use into a panic, a reused slot is
  another valid object, but `obj_klass` of a freed slot reads the free-list link.
* **H2, the mint contract**: the fiber switch (fiber.rs) and the collector's own access to
  contexts (`visit_ctx`) are trusted code.
* **The old accessors still exist.** A converted module cannot name them, but it can call a
  function of an unconverted module that uses them. While a converted module holds a payload
  borrow it must only call header functions and `std`; see step 4 of the recipe.
* **Allocators that do not take the context** (`str_new`, `hash_new`, `float_new`,
  `crate::gc::alloc` itself: 71 call sites of `gc::alloc` in 28 files). Calling one under a
  live payload borrow compiles, and a collection can then run under the borrow. Converted
  modules use the `Ctx` forms (the free names are banned there); the hole closes for good when
  `gc::alloc` is private to heap.rs.
* **Values in Rust-owned buffers.** `let mut v = c.ary(slf).to_vec(); sort(c, &mut v, ..)` is
  safe Rust and unrooted. The pilot keeps the existing convention (a keeper Array, recipe
  pattern 7). A precise or moving collector needs these buffers to be Ruby Arrays or
  registered `Cell` slots; that is roadmap section 2, not this change.

### Guard rail for the migration: `heap::no_legacy`

A converted module adds `use crate::heap::no_legacy::*;`. That module defines dummies named
`rary`, `obj`, `vm`, `ary_new`, `str_new`, ... so each of those names then has two glob
candidates and using one is `error[E0659]: 'rary' is ambiguous`. `tools/heap_audit.rb` checks
the same by text (it also catches a path such as `crate::object::rary`) and prints the
per-file counts used below.

## Alternatives

| design | result | cost |
|---|---|---|
| **(a) context-borrowing accessors** | chosen | run time: the kind check only (table below). Churn: every call site (mechanical: 206 sites in the pilot's two files). The compiler finds every site that held a reference across a call. |
| (a) without the kind check (`unreachable_unchecked`, as `rary`) | rejected, measured (`nokind`) | saves 0 to 2.1 % instructions (`ary` AOT 1953 M -> 1913 M, mapreduce 3608 M -> 3561 M, nothing on the index benchmarks); the accessor would be a safe function with undefined behaviour on a wrong argument. |
| (b) interior mutability: a borrow flag in `RObj.flags`, accessors return guards (`RefCell`) | rejected, measured (`refcell`) | against (a) built from the same tree, baked code: `ary` +96 % cycles (+13 % instructions), sieve +85 %, fannkuch +66 %, sort +13 %; interpreter +0 to +7 %. A violation is a panic in production instead of a compile error, and the guard says nothing about allocation: a guard held across a GC point is fine for `RefCell` and wrong for a moving collector. Churn is not avoided, only hidden: see the two findings below. |
| (b') `Cell<Value>` elements (`Vec<Cell<Value>>`) behind `&RObj` | rejected by argument | makes element writes under a shared reference legal, and a collector may then update slots under it; does nothing for `push` (the buffer moves under the reference), so a token or a flag is still needed for every structural change; and every `Vec<Value>` API in the builtins (`sort_by`, `extend_from_slice`, `iter().copied()`) changes type. The header fields do get this treatment (by-value access). |
| (c) closure-scoped access `c.with_ary(v, \|a\| ..)` taking the token | rejected by argument | the same machine code as (a) after inlining (`with_ary(v, f)` is `f(self.ary(v))`), the same rule, plus a closure at every site: `?` and `return` do not cross it (array_methods.rs has 229 `?`), and the closure can only read through `c`. |
| (c') closure-scoped access without a token (`with_ary(v, \|a\| ..)` as a free function) | rejected by argument | unsound alone: the closure can call `with_ary_mut(v, ..)`. It needs (b)'s flag, so it is (b) plus nesting. |
| typed handles (`Ary(Value)` checked once, unchecked accessors) | not done | would bring (a) to `nokind`'s numbers where a function touches the same array repeatedly. Sound only if a body never changes kind and the handle cannot outlive the object; worth it only if the check shows up in time, which it does not here. |

Two findings from the (b) emulation (`emu_variants.py refcell`: the pilot's `c.ary`, `c.ary_mut`
and `c.ary_modify` return guards that count readers and mark a writer in `RObj.flags`, and release
on drop; the guards are `'static`, as a tokenless accessor's would be):

* **Its first run panicked** ("already borrowed", benchmark aryidx). `Array#initialize` has
  `let v = c.ary_modify(slf)?; v.clear(); ...` and, further down, `c.ary_push(slf, x)` in the
  block loop. Under (a) the first borrow ends at `v.clear()`. A guard lives to the end of its
  scope, so the write flag was still set when the push came. Nothing at compile time points at
  such a place. Making the same guards borrow the context (`emu_variants.py guardscope`, only
  type-checked) turns them into errors: 93 in array_methods.rs. 10 are a write guard alive
  across another use of the context (the kind that panicked). 83 are a read guard alive across
  a call that takes the context mutably: 38 in the `while let Some(&x) = c.ary(slf).get(i)`
  loops, whose scrutinee temporary lives through the loop body, about 30 a temporary alive
  during an allocation or a push in the same statement. Under (b) each of these is either a
  latent panic or a reference across a GC point, and all of them compile.
* **The cycle cost is far above the instruction cost** in baked code (`ary`: +13 % instructions,
  +96 % cycles, spread 0.6 %). The likely mechanism, not isolated with counters: the flag is a
  load, an add and a store on the same header word, twice per access (borrow and release), so
  successive accesses to one array form a dependency chain through memory (each load waits
  for the previous store to be forwarded), and a loop that does little else per element runs
  at the speed of that chain. It fits the pattern: large where one array is hit in a tight
  loop (ary, sieve, fannkuch), absent where each access is separated by a call or a dispatch
  (cmpsort, mapreduce, the interpreter).

## Measurements

`bench.sh` (trials directory): each variant is a snapshot of `saruby` + `libsaruby.so`; per
benchmark the program is baked once per variant, then the variants are run interleaved inside
each repetition under `perf stat -e cycles:u,instructions:u`. 5 repetitions; the tables give the
minimum, and for cycles the spread (max-min)/min. Millions. Instruction counts repeat to 0.1 %.
`interp` is `--plain`, `AOT` runs the baked code. The machine is shared; the load average
during each run is given.

### Before and after

`base` = 5453803 (the commit before this work), `final` = 9bfce1a. Default build. Load average
1.5 to 2.1 (run5).

| interp | base ins | final ins | base cyc (spread) | final cyc (spread) |
|---|---|---|---|---|
| ary | 5934 | 5971 (+0.6 %) | 1794 (2.8 %) | 1806 (0.4 %) +0.7 % |
| aryidx | 142 | 141 (-0.7 %) | 67 (5.2 %) | 65 (8.5 %) -2.8 % |
| array_access | 9696 | 9725 (+0.3 %) | 2765 (1.0 %) | 2868 (4.0 %) +3.7 % |
| sort | 1529 | 1529 (-0.0 %) | 514 (5.0 %) | 510 (2.4 %) -0.7 % |
| cmpsort | 3352 | 3361 (+0.3 %) | 886 (4.2 %) | 877 (3.7 %) -1.0 % |
| mapreduce | 4595 | 4644 (+1.1 %) | 1173 (1.2 %) | 1252 (2.5 %) +6.7 % |
| nesteddata | 1131 | 991 (-12.4 %) | 336 (4.2 %) | 309 (3.1 %) -8.0 % |
| gcchurn | 5739 | 5411 (-5.7 %) | 1566 (2.4 %) | 1521 (5.8 %) -2.9 % |
| fannkuch | 6206 | 6145 (-1.0 %) | 2233 (3.0 %) | 2200 (8.8 %) -1.5 % |
| sieve | 3559 | 3543 (-0.4 %) | 1154 (11.4 %) | 1187 (10.0 %) +2.9 % |

| AOT | base ins | final ins | base cyc (spread) | final cyc (spread) |
|---|---|---|---|---|
| ary | 1914 | 1953 (+2.1 %) | 495 (4.9 %) | 475 (4.6 %) -3.9 % |
| aryidx | 79 | 78 (-0.7 %) | 40 (1.0 %) | 40 (1.1 %) -1.6 % |
| array_access | 3186 | 3230 (+1.4 %) | 735 (2.4 %) | 747 (1.9 %) +1.6 % |
| sort | 788 | 787 (-0.1 %) | 252 (6.6 %) | 253 (0.4 %) +0.5 % |
| cmpsort | 2525 | 2531 (+0.2 %) | 655 (3.8 %) | 644 (4.2 %) -1.7 % |
| mapreduce | 3559 | 3608 (+1.4 %) | 871 (3.5 %) | 870 (2.5 %) -0.1 % |
| nesteddata | 819 | 724 (-11.7 %) | 256 (0.9 %) | 246 (3.8 %) -3.6 % |
| gcchurn | 2124 | 2107 (-0.8 %) | 735 (2.0 %) | 719 (1.0 %) -2.1 % |
| fannkuch | 1711 | 1692 (-1.1 %) | 404 (5.1 %) | 391 (3.8 %) -3.3 % |
| sieve | 970 | 955 (-1.6 %) | 235 (0.7 %) | 236 (4.1 %) +0.3 % |

Every benchmark prints the same output in every variant, interpreter and AOT.

How to read it, row by row:

* **ary, array_access, mapreduce, cmpsort (+0.2 to +2.1 % instructions): the checks.** `nokind`
  (the same tree with `unreachable_unchecked` in `c.ary` / `c.ary_mut`) gives back exactly
  these: ary 5971 -> 5933 / 1953 -> 1913, mapreduce 4644 -> 4597 / 3608 -> 3561, cmpsort
  3361 -> 3359 / 2531 -> 2528, array_access -30 M in both modes. It is two instructions (test
  the tag bits, compare the kind byte) per access that the compiler cannot merge with an
  earlier one: per element in `Array#sum` and the block loops, per call in `Array#push`.
  In `a[i]`, `a[i] = v` and `a << x` the node's own `is_array` test subsumes it (checked in
  the disassembly: no test added).
* **aryidx, fannkuch, sieve (-0.4 to -1.6 %): how `a[i]` is written**, not the design. Converted
  as it stood, the node came out 3 instructions longer per read (array_access AOT +4.2 %,
  fannkuch +4.1 %, sieve +2.0 %; `nokind` has the same counts on the last two): with a slice in hand LLVM folded
  `if i < 0 { i += len }` into `sar; and; add` on the common path instead of a branch around
  it. Written as `if let Some(&x) = v.get(i as usize) { return Ok(x) }` with the negative case
  after it, the read is one unsigned compare and these benchmarks end below the baseline.
* **nesteddata, gcchurn (-5.7 to -12.4 %): not an effect of this change.** See below.
* **Cycles.** In baked code -3.9 % to +1.6 %, with spreads of 1 to 5 %: no cost visible. In
  the interpreter mapreduce shows +6.7 % and array_access +3.7 %. Repeated with 10 repetitions
  and two more builds (run6, load 1.5 to 1.8): mapreduce `final` +6.3 %, but `pilot3` (the
  commit before the `a[i]` rewrite: the same 4644 M instructions, mapreduce never executes
  `a[i]`) +2.5 %, and `nokind` +1.8 % with base's instruction count. Two builds executing the
  same instructions differ by 3.7 % in cycles, so this is code placement in the interpreter
  (roadmap.md, "Open measurements", records the same for poly), not the accessors. AOT in
  the same run: mapreduce +0.2 %, array_access +2.0 %.

### The allocator accident (nesteddata, gcchurn)

A 12 % drop from a change that adds checks is not credible, so it was taken apart:

* GC counts and allocation counts are identical (54 collections and 2000498 objects for
  nesteddata in both builds).
* `perf record -e instructions:u`: in `base`, `RawVecInner::try_allocate_in` and a `RawVec`
  drop are separate symbols (9 % of samples together); in the new build they do not exist.
* `objdump` of `hash::hash_new_capa`: `base` calls `__rust_alloc` through the GOT; the new
  build has the global allocator (`kalloc`, its `refill` and `malloc` calls) inlined into it.
  `libsaruby.so` grew from 24.8 MB to 26.8 MB: it is inlined at every allocation site.
* It appears at one commit of this series and has nothing to do with what that commit means:
  a build of the tree at "heap access core" does not have it, adding only the new array.rs and
  the move of the context-free `ary_*` functions to object.rs does. Presumably (not
  traced into LLVM): with 16 codegen units the crate is optimized by ThinLTO across units, and
  whether `__rust_alloc` is imported into its callers is a threshold decision that the
  regrouping of functions tipped.
* Control: both trees built with `CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1`, where both inline the
  allocator (run4, instruction counts; the machine was loaded during that run, cycles unusable):
  nesteddata 988 -> 990 M interp, 718 -> 719 M AOT; gcchurn 5426 -> 5418 M, 2123 -> 2123 M.
  The other eight benchmarks move as in the table above (ary +0.6 % / +2.1 %, mapreduce
  +1.1 % / +1.4 %, ...).

So the design is neutral on allocation, and the default build of the baseline leaves about
12 % of nesteddata's instructions (5.7 % of gcchurn's in the interpreter) on the table through
an un-inlined allocator. That is worth fixing on purpose (one codegen unit, or allocation
paths that call `kalloc` directly) instead of by accident; it is independent of this work.

### The alternatives

Same run for all (run3, load 5.5 to 7.3). `(a)` here is the series before the `a[i]` rewrite
(`pilot3`); `nokind` and `refcell` are that tree with `emu_variants.py` applied. Instructions,
and for `refcell` the minimum cycles against `(a)` in the same run.

| AOT | base ins | (a) ins | nokind ins | refcell ins | (a) cyc (spread) | refcell cyc (spread) |
|---|---|---|---|---|---|---|
| ary | 1914 | 1953 | 1913 | 2213 | 474 (2.5 %) | 929 (0.6 %) +96 % |
| aryidx | 79 | 80 | 79 | 84 | 43 (2.2 %) | 46 (4.0 %) +7 % |
| array_access | 3186 | 3320 | 3290 | 3470 | 739 (1.5 %) | 771 (1.8 %) +4 % |
| sort | 788 | 787 | 787 | 802 | 260 (3.0 %) | 295 (1.5 %) +13 % |
| cmpsort | 2525 | 2531 | 2528 | 2534 | 673 (4.7 %) | 686 (8.0 %) +2 % |
| mapreduce | 3559 | 3608 | 3561 | 3691 | 897 (6.0 %) | 906 (4.9 %) +1 % |
| nesteddata | 819 | 724 | 724 | 729 | 250 (8.0 %) | 252 (10.3 %) +1 % |
| gcchurn | 2124 | 2107 | 2107 | 2107 | 734 (2.7 %) | 731 (2.1 %) -0 % |
| fannkuch | 1711 | 1781 | 1781 | 1960 | 400 (4.0 %) | 663 (1.0 %) +66 % |
| sieve | 970 | 990 | 990 | 1097 | 238 (6.1 %) | 440 (0.8 %) +85 % |

| interp | base ins | (a) ins | nokind ins | refcell ins | (a) cyc (spread) | refcell cyc (spread) |
|---|---|---|---|---|---|---|
| ary | 5934 | 5971 | 5933 | 6233 | 1813 (3.5 %) | 1852 (3.3 %) +2 % |
| aryidx | 142 | 144 | 143 | 147 | 67 (5.8 %) | 71 (5.5 %) +6 % |
| array_access | 9696 | 9830 | 9800 | 9995 | 2787 (3.4 %) | 2831 (2.2 %) +2 % |
| sort | 1529 | 1529 | 1529 | 1544 | 510 (4.3 %) | 520 (2.4 %) +2 % |
| cmpsort | 3352 | 3361 | 3359 | 3365 | 913 (2.3 %) | 918 (0.9 %) +1 % |
| mapreduce | 4595 | 4644 | 4597 | 4727 | 1211 (3.1 %) | 1214 (2.4 %) +0 % |
| nesteddata | 1131 | 996 | 996 | 999 | 311 (9.0 %) | 309 (11.2 %) -1 % |
| gcchurn | 5739 | 5411 | 5410 | 5410 | 1535 (6.5 %) | 1537 (3.3 %) +0 % |
| fannkuch | 6206 | 6286 | 6286 | 6463 | 2183 (3.3 %) | 2325 (2.7 %) +7 % |
| sieve | 3559 | 3578 | 3578 | 3659 | 1219 (9.6 %) | 1235 (5.5 %) +1 % |

`Body` as `repr(u8)` alone (`repr`, the baseline plus that one line, same run) is within 0.2 %
of the baseline in instructions on all ten, both modes.

## Verification

At 9bfce1a against 5453803 (`base`); logs in the trials directory.

* `make` and `make FEATURES=raw_stack`: no warnings. Each of the three earlier commits of the
  series also builds without warnings (default configuration, from a clean copy) and gives the
  baseline's output on the 67 array files below.
* `ruby tools/heap_audit.rb`: exit 0 (no converted file names an old accessor).
* The guard rails do fire. Appending
  `fn t(v: Value) -> usize { rary(v).len() + vm().sd_hits + ary_new().raw() as usize }` to
  array_methods.rs gives three `E0659` (`rary`, `vm`, `ary_new` are ambiguous); appending
  `fn t(c: &mut Ctx, v: Value) -> RV { let a = c.ary(v); let x = c.ary_new(); Ok(if a.is_empty() { x } else { a[0] }) }`
  gives `E0502: cannot borrow *c as mutable because it is also borrowed as immutable`.
* Differential tests against CRuby, `run_specs.rb --jobs 4 --timeout 15 --pattern '*.rb' --diff ruby
  --dir t/hand` (1702 files), run alternately, TOTAL row:

  | build | run | PASS | FAIL | CRASH | WCRASH | TIMEO | REFTO | load average at start |
  |---|---|---|---|---|---|---|---|---|
  | base | 1 | 23069 | 10422 | 0 | 5 | 17 | 0 | 2.0 |
  | 9bfce1a | 1 | 23069 | 10419 | 0 | 7 | 15 | 0 | 5.8 |
  | base | 2 | 23068 | 10414 | 0 | 5 | 18 | 0 | 6.9 |
  | 9bfce1a | 2 | 23069 | 10415 | 0 | 7 | 15 | 0 | 12.4 |

  PASS is equal; FAIL moves by a few lines between two runs of the same build
  (`ary_mutate_str_bits_1020.rb` line 9 comes and goes in both). The one systematic difference:
  `combinatorics_stream_1018.rb` and `cruby_test_ruby_fixes_1017.rb` are TIMEO with `base` and
  WCRASH with the new build, in both runs. Run alone, **both builds die the same way** on both
  files: killed by the kernel (status 137) after 17 to 21 s at a peak of 22 GB resident
  (`/usr/bin/time`: 22.5 / 22.7 GB and 22.3 / 22.3 GB). They are memory-exhaustion cases that
  neither build passes, and the label says only whether the kill or the 15 s limit came first.
  Under a 6 GB address-space limit the two builds fail at the same resident size (4.82 GB,
  2.55 GB); the new one gets there 2 to 6 % sooner on the first file (5.00 to 5.04 s against
  5.14 to 5.34 s, three runs), which fits the faster allocation of this build (the allocator
  accident above) pulling the kill under 15 s. That last step is an inference: confirming it
  means running two 22 GB processes side by side again on a shared machine, which was not done.
* Per-file output (stdout, stderr and exit status) of the 67 files `t/hand/array*.rb` and
  `ary_*.rb`, 60 s limit: 66 identical between `base` and the new build. The 67th,
  `ary_shared_slice_1115.rb`, prints the result of `shuffle!(random: Random.new(1))`, which
  saruby does not seed: it differs between two runs of the same binary.
* The same 67 files under `SARUBY_GC_STRESS=1` (a collection at every allocation; a 2000-element
  `map` reports GC.count 2003 instead of 0, so the switch is live): `base` against the new
  build 66 identical plus the same nondeterministic file; and for each build, stress against no
  stress, the same. No array file depends on the stress setting before or after.
* The ten benchmarks print identical output in every variant measured, interpreter and AOT.

## Recipe: converting one module

`ruby tools/heap_audit.rb` shows what a file still uses. For a module `m.rs`:

1. **Turn the names off.** At the top:

   ```rust
   #![forbid(unsafe_code)]            // if the module has unsafe of its own, deal with that first or leave this for later
   #[allow(unused_imports)]
   use crate::heap::no_legacy::*;     // the old accessors cannot be named here
   ```

   `cargo check` now lists every use (E0659). If other modules glob-import *through* this one
   (`pub use crate::object::*` as in array.rs), the ban must not sit next to that re-export:
   put the module's code in an inner `mod imp { .. }` and `pub use imp::*` (see array.rs).
2. **Replace, mechanically.**

   | old | new |
   |---|---|
   | `rary(v)` read | `c.ary(v)` |
   | `rary(v)` write | `c.ary_mut(v)`; with the frozen check `c.ary_modify(v)?` |
   | `ary_len(v)`, `ary_get(v, i)`, `ary_push(a, x)`, `ary_new()`, `ary_new_from(s)`, `ary_from_vec(v)`, `ary_new_capa(n)` | the same names as methods: `c.ary_len(v)`, ... |
   | `ary_to_vec(v)`, `rary(v).clone()` | `c.ary(v).to_vec()` |
   | `rstr(v)`, `str_bytes(v)` | `c.rstring(v)` / `c.rstring_mut(v)`, `c.str_bytes(v)` |
   | `rhash(v)`, `rclass(v)`, `rproc(v)`, `rmethod(v)`, `rbig(v)`, `ivars_of(v)` | `c.hash(v)` / `c.hash_mut(v)`, `c.class(v)` / `c.class_mut(v)`, `c.proc_data(v)`, `c.method_obj(v)`, `c.big(v)`, `c.ivars(v)` / `c.ivars_mut(v)` |
   | `rdata::<T>(v)` and local helpers `fn th(v) -> &'static mut T` | `c.data::<T>(v)` / `c.data_mut::<T>(v)`; helpers become `fn th(c: &Ctx, v: Value) -> &T` and `fn th_mut(c: &mut Ctx, v: Value) -> &mut T` |
   | `obj(v).klass`, `obj(v).klass = k` | `obj_klass(v)`, `obj_set_klass(v, k)` |
   | `obj(v).flags & F`, `\|= F`, `&= !F` | `obj_flags(v) & F`, `obj_set_flags(v, F)`, `obj_clear_flags(v, F)` |
   | `matches!(obj(v).body, Body::X(..))` | `is_x(v)` (add a line to `kind_test!` in heap.rs if missing) |
   | `vm().field` | `c.vm().field`; a write is `c.vm_mut().field = ..` |
   | `str_new(b)`, `str_from_vec(v)`, `hash_new()`, `float_new(d)`, `gc::alloc(k, body)` | `c.str_new(b)`, ..., `c.alloc(k, body)`. For an allocator not wrapped yet, add the `&mut self` form to the block at the end of heap.rs and the free name to `no_legacy`. |
   | a function without `c` that touches a payload | give it `c: &Ctx` (reads) or `c: &mut Ctx` (writes, allocates) and pass it from the callers; a boot-time `init()` takes what it needs as arguments (`array_methods::init(k)`) |

3. **Fix what the borrow checker reports.** Each error is a place where a reference was held
   across something that takes the context. The patterns met in the pilot are below.
4. **Check what it cannot report**, by reading the module once:
   * every `let x = c.ary(..)` / `c.ary_mut(..)` / `c.str_bytes(..)` binding (a borrow with
     a name): between it and its last use, only `std` methods, header functions (`is_*`,
     `obj_*`) and other `c.` reads. No free function of an unconverted module that could touch
     a payload or allocate.
   * every `Vec<Value>` filled from a payload: if a call taking `c` happens while it is alive,
     its Values need a root (pattern 7).
   * after a call that runs Ruby code, re-check what the code before it checked (frozen:
     `c.ary_modify(slf)?` again; length: re-read).
5. **Module state.** A `static mut` holding Values becomes a `Vm` field (visited in
   `vm::visit_roots`), read with `c.vm()`, written with `c.vm_mut()`.
6. `make` (no warnings), `make FEATURES=raw_stack`, `ruby tools/heap_audit.rb` (exit 0), the
   differential tests of the module's area, and the same files under `SARUBY_GC_STRESS=1`.

### Patterns

1. **Element or length**: `c.ary_len(a)`, `c.ary_get(a, i)` (nil beyond the end),
   `c.ary(a)[i]`, `c.ary(a).get(i).copied()`.
2. **Iterating while calling a block.** The block can change the array, so CRuby re-reads the
   length at every step; so must we, and the borrow rule forces it. Length test and element
   read are one borrow:

   ```rust
   let mut i = 0;
   while let Some(&x) = c.ary(slf).get(i) { yield1(c, blk, x)?; i += 1; }
   ```

   Backwards (`reverse_each`, `rindex`): `let mut i = c.ary_len(slf); while i > 0 { i -= 1;
   if i >= c.ary_len(slf) { i = c.ary_len(slf); continue; } let x = c.ary_get(slf, i); .. }`.
3. **Iterating without calling out**: one borrow for the whole loop.
   `c.ary(slf).iter().copied().filter(|x| *x != NIL).collect()`,
   `v.iter().all(|x| is_string(*x))` (a header test under a borrow is fine),
   `v.sort_by(|a, b| c.str_bytes(*a).cmp(c.str_bytes(*b)))`.
4. **Building a result while reading.**
   * No call in between: collect into a `Vec`, then hand it to the allocator. The argument is
     evaluated before `c` is borrowed mutably, so it fits in one expression:
     `c.ary_from_vec(c.ary(slf)[st..e].to_vec())`. The Vec's Values are roots during the
     allocation (it is the pending body).
   * A block per element: the result must be rooted, so it is a Ruby Array from the start:
     `let out = c.ary_new(); while let Some(&x) = c.ary(slf).get(i) { let y = yield1(c, blk, x)?; c.ary_push(out, y); i += 1; }`.
5. **Writing an element after calling out**: `let y = yield1(c, blk, x)?; c.ary_modify(slf)?;
   if i < c.ary_len(slf) { c.ary_mut(slf)[i] = y; }` (frozen and length re-checked: the block
   ran).
6. **Two bodies.** Read both: `v.extend_from_slice(c.ary(o))` with `v` a local Vec, or
   `c.ary(a).iter().chain(c.ary(b).iter())`. Write one from another:

   ```rust
   let (dst, others) = c.ary_mut_and(slf);
   if let Some(src) = others.ary(o) { dst.clear(); dst.extend_from_slice(src); }   // None: o is slf, nothing to do
   ```

   When the source has to be a snapshot anyway (`a[1, 2] = a`), copy out first:
   `let repl = c.ary(val).to_vec(); c.ary_mut(slf).splice(st..e, repl);`.
7. **Sort with a user comparator** (any algorithm that needs a mutable slice across Ruby
   calls). Copy out, keep the elements alive with a Ruby Array, sort the copy, write back:

   ```rust
   c.ary_modify(slf)?;
   let mut v = c.ary(slf).to_vec();
   let keep = c.ary_new_from(&v);          // the block may empty slf; v is not scanned
   sort_array(c, &mut v, blk)?;            // cmp: &mut dyn FnMut(&mut Ctx, Value, Value) -> ..
   std::hint::black_box(keep);
   *c.ary_modify(slf)? = v;                // the block may have frozen slf
   ```

   The comparator gets `c` as an argument (`&mut |c, a, b| rb_cmp(c, a, b)`); it cannot capture
   it, since the sort itself holds it. CRuby does the same thing with a hidden `tmp` array.
8. **Frozen check and write**: `c.ary_modify(v)?` returns the `&mut Vec` or raises.
   `let v = c.ary_modify(slf)?; .. num_to_long(c, a[0])? ..; let v = c.ary_mut(slf);` is the
   shape when a conversion sits between two writes: the first borrow ends at its last use, the
   second is a new one.
9. **Raising with a class from the VM**: `err(c, c.vm().c_index_error, format!(..))`.
10. **Recursion** (`flatten_into`, `ary_join`, `ary_inspect`): pass `Value`s down, never
    references; each level takes its own borrows.
11. **A closure that needs `c` and locals** (`let r: Result<(), Esc> = (|| { .. yield1(c, ..) .. })();`
    to run cleanup after): works unchanged, the closure borrows `c` for its own duration.

## What is still open

* The call sites below.
* Allocation behind the context's back (71 `gc::alloc` sites), until `gc::alloc` is reachable
  only through `Ctx::alloc`.
* `Ctx::frame() -> &'static mut Frame` and the raw `*const Block` / `*const MethodEntry` /
  `*const Cref` graph: the frame chain, a separate surface (safe_stack.md lists it too).
* cfunc arguments `&[Value]`, `rdata` payloads holding Values, `Vec<Value>` scratch buffers:
  rooting, roadmap section 2.

## Remaining work per file

`ruby tools/heap_audit.rb --md` at 9bfce1a. Columns: call sites of each old accessor
(`rstr` includes `str_bytes`, `rproc` includes `rmethod`, `ary_*` are the context-free array
functions of object.rs); `sum`; functions of the file that return `&'static mut`; `static mut`
declarations; `unsafe` occurrences (comments excluded). Generated `src/node_*.rs` and
`src/prism_ast.rs` are not counted; `node.def`, their source, is.

| file |    obj |   rary |   rstr |  rhash | rclass |  rproc |   rbig |  rdata | ivars_of |  ary_* |     vm |   sum | fn->&'static mut | static mut | unsafe | state |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| src/aot.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      1 |     1 |                0 |          1 |      4 |  |
| src/array.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/bignum.rs |      1 |      0 |      0 |      0 |      0 |      0 |      1 |      0 |      0 |      0 |      1 |     3 |                0 |          0 |      1 |  |
| src/builtins/array_methods.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/builtins/binding.rs |      0 |      0 |      1 |      0 |      0 |      1 |      0 |      1 |      0 |      3 |      9 |    15 |                1 |          0 |      9 |  |
| src/builtins/complex.rs |      1 |      0 |      4 |      0 |      1 |      0 |      0 |      2 |      0 |      8 |      4 |    20 |                0 |          0 |      0 |  |
| src/builtins/encoding.rs |      1 |      0 |     16 |      0 |      0 |      0 |      0 |      9 |      0 |     10 |      7 |    43 |                0 |          5 |     10 |  |
| src/builtins/enum_native.rs |      1 |     10 |      1 |      0 |      0 |      0 |      0 |      0 |      0 |     63 |      7 |    82 |                0 |          0 |      1 |  |
| src/builtins/enumerator.rs |      3 |      1 |      0 |      0 |      1 |      3 |      0 |      2 |      0 |     41 |     13 |    64 |                1 |          0 |      1 |  |
| src/builtins/env.rs |      1 |      0 |      3 |      0 |      0 |      0 |      0 |      0 |      0 |      4 |      2 |    10 |                0 |          0 |      4 |  |
| src/builtins/exception.rs |      0 |      0 |     12 |      0 |      2 |      0 |      0 |      0 |      0 |     14 |     15 |    43 |                0 |          0 |      0 |  |
| src/builtins/extras.rs |      0 |      0 |      1 |      0 |      3 |      0 |      0 |      0 |      0 |      9 |     13 |    26 |                0 |          0 |     13 |  |
| src/builtins/file.rs |      0 |      0 |      4 |      0 |      1 |      0 |      0 |      6 |      0 |     13 |     13 |    37 |                0 |          1 |     25 |  |
| src/builtins/hash_methods.rs |     10 |      2 |      2 |     16 |      0 |      2 |      0 |      0 |      0 |     48 |      9 |    89 |                0 |          0 |      2 |  |
| src/builtins/io.rs |      0 |      0 |     23 |      0 |      1 |      0 |      0 |      1 |      0 |     11 |     22 |    58 |                3 |          1 |     32 |  |
| src/builtins/json.rs |      6 |      0 |     10 |      0 |      0 |      0 |      1 |      0 |      0 |      6 |     15 |    38 |                0 |          2 |      8 |  |
| src/builtins/kernel.rs |      2 |      0 |     18 |      0 |      5 |      0 |      1 |      0 |      0 |     16 |     66 |   108 |                0 |          1 |     14 |  |
| src/builtins/marshal.rs |      9 |      4 |     13 |      1 |     10 |      0 |      0 |      0 |      0 |     26 |     17 |    80 |                0 |          0 |      0 |  |
| src/builtins/math.rs |      0 |      0 |      0 |      0 |      0 |      0 |      1 |      0 |      0 |      3 |      4 |     8 |                0 |          0 |      0 |  |
| src/builtins/misc.rs |      1 |      2 |      9 |      1 |      1 |      0 |      1 |      4 |      0 |     31 |      3 |    53 |                0 |          2 |      6 |  |
| src/builtins/mod.rs |      3 |      0 |      0 |      0 |     15 |      0 |      0 |      0 |      0 |      4 |      1 |    23 |                0 |          0 |      2 |  |
| src/builtins/module.rs |      4 |      0 |      0 |      0 |      4 |      0 |      0 |      0 |      0 |      0 |      0 |     8 |                0 |          0 |      1 |  |
| src/builtins/numeric_methods.rs |      0 |      0 |      4 |      0 |      0 |      0 |     15 |      0 |      0 |     27 |     30 |    76 |                0 |          0 |      0 |  |
| src/builtins/object.rs |     22 |      8 |      5 |      0 |     79 |      2 |      0 |      0 |      0 |     42 |     54 |   212 |                0 |          1 |     44 |  |
| src/builtins/pp.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      2 |      0 |     2 |                0 |          0 |      0 |  |
| src/builtins/prelude.rs |      0 |      0 |      0 |      0 |      1 |      0 |      0 |      0 |      0 |      1 |      1 |     3 |                0 |          0 |      0 |  |
| src/builtins/proc.rs |      4 |      2 |      0 |      0 |      4 |     37 |      0 |      0 |      0 |     39 |     16 |   102 |                0 |          0 |     30 |  |
| src/builtins/process.rs |      0 |      3 |      5 |      0 |      0 |      0 |      0 |      0 |      0 |     25 |     10 |    43 |                0 |          3 |     54 |  |
| src/builtins/random.rs |      1 |      0 |      2 |      0 |      2 |      0 |      0 |      9 |      0 |      0 |      9 |    23 |                1 |          1 |      2 |  |
| src/builtins/range_methods.rs |      3 |      0 |      6 |      0 |      1 |      0 |      0 |      0 |      0 |     17 |     11 |    38 |                0 |          0 |      0 |  |
| src/builtins/rational.rs |      1 |      0 |      4 |      0 |      1 |      0 |      5 |      2 |      0 |      8 |     10 |    31 |                0 |          0 |      0 |  |
| src/builtins/regexp_methods.rs |      4 |      1 |     15 |      0 |      2 |      0 |      0 |      0 |      0 |      7 |      5 |    34 |                1 |          0 |      0 |  |
| src/builtins/sprintf.rs |      0 |      0 |      6 |      2 |      0 |      0 |      0 |      0 |      0 |      0 |     13 |    21 |                0 |          0 |      0 |  |
| src/builtins/string_methods.rs |     10 |      1 |     66 |      0 |      0 |      0 |      3 |      0 |      0 |     30 |     36 |   146 |                1 |          0 |      0 |  |
| src/builtins/stringio.rs |      0 |      0 |     29 |      0 |      1 |      0 |      0 |      2 |      0 |      2 |     12 |    46 |                3 |          0 |      0 |  |
| src/builtins/strscan.rs |      0 |      0 |     17 |      0 |      1 |      0 |      0 |      4 |      0 |      4 |      3 |    29 |                1 |          2 |      3 |  |
| src/builtins/structs.rs |      1 |      6 |      7 |      0 |      7 |      0 |      0 |      0 |      0 |     31 |      7 |    59 |                0 |          0 |      0 |  |
| src/builtins/symbol_methods.rs |      1 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      2 |      1 |     4 |                0 |          0 |      0 |  |
| src/builtins/sys.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      6 |  |
| src/builtins/thread.rs |      0 |     10 |      1 |      0 |      4 |      0 |      0 |      6 |      0 |     29 |     25 |    75 |                4 |          8 |     40 |  |
| src/builtins/time.rs |      0 |      0 |      7 |      0 |      1 |      0 |      0 |      4 |      0 |      2 |     10 |    24 |                1 |          0 |      2 |  |
| src/compile.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      4 |  |
| src/fiber.rs |      0 |      1 |      0 |      0 |      1 |      1 |      0 |      1 |      0 |      2 |     10 |    16 |                1 |          5 |     21 |  |
| src/gc.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      2 |     2 |                1 |          3 |     13 |  |
| src/hash.rs |      9 |      2 |      2 |     25 |      0 |      0 |      2 |      0 |      0 |      7 |      7 |    54 |                1 |          1 |      4 |  |
| src/heap.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |     12 |  |
| src/kalloc.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          5 |     12 |  |
| src/lib.rs |      0 |      1 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      2 |      8 |    11 |                0 |          0 |      2 |  |
| src/numeric.rs |      1 |      0 |      0 |      0 |      0 |      0 |      1 |      0 |      0 |      0 |      1 |     3 |                0 |          0 |      0 |  |
| src/object.rs |      8 |      4 |      1 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      4 |    17 |                7 |          1 |      7 |  |
| src/prism.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      1 |  |
| src/range.rs |      2 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      2 |     4 |                0 |          0 |      0 |  |
| src/regexp.rs |      0 |      0 |      9 |      0 |      0 |      0 |      0 |      2 |      0 |      2 |      4 |    17 |                2 |          0 |      0 |  |
| src/rt.rs |     15 |     50 |      9 |      4 |     30 |      1 |      0 |      0 |      0 |     14 |     52 |   175 |                0 |          2 |     62 |  |
| src/sbuf.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |     10 |  |
| src/stack_raw.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      5 |  |
| src/string.rs |      3 |      0 |     27 |      0 |      0 |      0 |      0 |      0 |      0 |      2 |      5 |    37 |                0 |          0 |      0 |  |
| src/symbol.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                1 |          1 |      2 |  |
| src/value.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |     11 |  |
| src/vm.rs |     20 |      3 |      2 |      5 |     60 |     10 |      0 |      1 |      1 |      7 |     53 |   162 |                2 |          2 |     67 |  |
| node.def |     11 |      0 |      2 |      0 |      0 |      0 |      0 |      0 |      0 |      3 |     11 |    27 |                0 |          0 |     22 |  |
| total |    159 |    111 |    343 |     54 |    239 |     57 |     31 |     56 |      1 |    617 |    634 |  2302 |               32 |         48 |    569 |  |
