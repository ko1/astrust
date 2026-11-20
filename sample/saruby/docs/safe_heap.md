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
* The String side followed (section "The String side"): string.rs, string_methods.rs,
  sprintf.rs, encoding.rs, stringio.rs, strscan.rs and symbol_methods.rs are converted and
  `forbid(unsafe_code)`; heap.rs gained one `unsafe` (`str_mut_and`).
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
  | `c.rstring(v) -> &RString`, `c.str_bytes(v) -> &[u8]`, `c.str_enc(v)`, `c.str_len(v)` | `c.rstring_mut(v)`; with the frozen check `c.str_modify(v)?` | String |
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
  `c.str_mut_and(dst) -> (&mut RString, Others<'_>)` is the same for a String
  (`others.rstring(src)`, `others.str_bytes(src)`); `c.str_append_enc(acc, s, enc)` in string.rs
  is `acc << s` on it, `s << s` included.
* Writing two: not needed so far (Array and String sides); the same construction with two
  exceptions when it is.

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
and `str_mut_and` (one more `&mut` under a shared borrow, justified by `Others`), `vm` /
`vm_mut`, `Token::mint`. 13 occurrences after the String side (12 in the pilot): `str_mut_and`
is the one addition, and it is `ary_mut_and` for another body kind, not a new kind of accessor.

Claim: under H1 to H3 below, a payload reference is never used after its payload was freed or
moved, and no `&mut` to a payload coexists with another reference to it.

* **H2 gives exclusivity.** Every payload reference is created by `body` / `body_mut` with the
  lifetime of a borrow of a `Ctx`. All `Ctx` values come from `Token::mint`, whose contract is
  that contexts run one at a time: a fiber's context is used only between switch-in and
  switch-out, and the context that resumed it is, for that time, mutably borrowed by the
  `resume` call on its stack. So at any moment every usable `&Ctx` / `&mut Ctx` is a reborrow
  of one context, and Rust's rules for that one value give "many readers or one writer" for the
  whole heap. `Others` is the only refinement: one writer plus readers of other objects
  (`ary_mut_and`, `str_mut_and`: the writer's payload and a reader's are payloads of distinct
  objects, hence disjoint allocations, and `Others` hands out nothing of the written one).
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
candidates and using one is `error[E0659]: 'rary' is ambiguous`. `heap::no_legacy_str` does
the same for the context-free String functions (`str_new_enc`, `str_enc`, `str_eq`,
`char_offsets`, ...: what is left of them lives in `str_legacy.rs`); it is a module of its own
so that the files converted before the String side keep compiling, and is meant to be folded
into `no_legacy` once they have switched to the `Ctx` methods. `tools/heap_audit.rb` checks
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

## The String side

2026-11-21, on top of the pilot. Scripts, logs and the test programs:
`~/ruby/src/trials/2026-11-21-saruby-safe-heap-string/`. `base` = 4a06252, `final` = 1c403de.

### What was converted

| file | old-accessor call sites | `fn -> &'static mut` | `static mut` | `unsafe` | state |
|---|---|---|---|---|---|
| src/string.rs | 37 -> 0 | 0 | 0 | 0 | `forbid(unsafe_code)` |
| src/builtins/string_methods.rs | 146 -> 0 | 1 -> 0 | 0 | 0 | `forbid(unsafe_code)` |
| src/builtins/sprintf.rs | 21 -> 0 | 0 | 0 | 0 | `forbid(unsafe_code)` |
| src/builtins/encoding.rs | 43 -> 0 | 0 | 5 -> 0 | 10 -> 0 | `forbid(unsafe_code)` |
| src/builtins/stringio.rs | 46 -> 0 | 3 -> 0 | 0 | 0 | `forbid(unsafe_code)` |
| src/builtins/strscan.rs | 29 -> 0 | 1 -> 0 | 2 -> 0 | 3 -> 0 | `forbid(unsafe_code)` |
| src/builtins/symbol_methods.rs | 4 -> 0 | 0 | 0 | 0 | `forbid(unsafe_code)` |
| src/str_legacy.rs (new) | 21 | 0 | 0 | 0 | the context-free String functions other modules still call |
| src/symbol.rs | 0 | 1 | 1 | 2 | not converted, see below |
| src/sbuf.rs | 0 | 0 | 0 | 10 | not touched: the byte buffer itself (a `Vec<u8>` with 23 inline bytes), `unsafe` by nature as `Vec` is |
| src/heap.rs | 0 | 0 | 0 | 12 -> 13 | `str_mut_and` |
| all files (`heap_audit.rb` total) | 2305 -> 2001 | 32 -> 27 | 48 -> 41 | 570 -> 558 | |

Added to heap.rs, in a block of its own: `Ctx::str_mut_and` (the one new `unsafe`) and
`Others::rstring`. Everything else the String side needs is safe code in string.rs built on
`rstring` / `rstring_mut` / `str_bytes`: `c.str_new`, `str_new_enc`, `str_from_vec[_enc]`,
`str_from_sbuf`, `str_with_capa`, `str_new_frozen`, `str_new_like`, `str_copy[_enc]`,
`str_sub`, `str_dup`, `str_enc`, `str_len`, `str_eq`, `str_cmp`, `str_hash`, `str_to_sym`,
`str_modify`, `str_append_enc`, `str_append_same_enc`, `char_offsets`, `str_char_len`,
`str_char_code`, `chars_of`, `str_concat_values`, `ary_of_strs`. The three allocator shims
`str_new` / `str_from_vec` / `str_from_vec_enc` left heap.rs: string.rs allocates through
`c.alloc`.

Edits outside these files: `vm.rs` (`Vm.enc_objs`, `Vm.c_string_scanner`,
`Vm.c_string_scanner_error`, `class_set_alloc`), `builtins/mod.rs` (the `init` calls, one
`ptr::write` for `enc_objs`), `lib.rs` (`mod str_legacy`), `prelude.rs` (`strscan::init(c)`),
`io.rs`, `misc.rs`, `regexp_methods.rs` (`encoding_value(c, ..)`), `array_methods.rs`
(`c.str_eq`), `tools/heap_audit.rb` (the `no_legacy_str` names).

`symbol.rs` keeps its table as a `static mut` behind `tab()`: `intern` / `sym_bytes` have 664
call sites without a context, so the table cannot borrow one, and the safe alternative that
needs no context, a `Mutex`, was measured and not taken: +8 % instructions on a loop of
`String#to_sym` / `Symbol#to_s` / `Symbol#length` (about 35 instructions per table access).
What it would take: the table owned by `Vm`, or names readable without a lock (chunks of
`OnceLock`) with the lock only in `intern`.

### What it costs

The kind check, as in the pilot, and nothing else by design. Counted in the disassembly
(`String#length`, `Ctx::str_eq`): `test $7,%dil; jne; cmpb $3,0x10(%rdi); jne`, four
instructions in two compare-and-branch pairs, per access that the compiler cannot merge with
an earlier test of the same value. `rstr` had none.

Instructions and cycles, `tools/icount.sh`-style runs under `perf stat -e
cycles:u,instructions:u`, 5 interleaved repetitions, minimum and (max-min)/min. Millions.
Run 1 of 2 (`logs/perf.final3`), load average 14.5 at the start, 9.4 at the end.

| interp | base ins | final ins | base cyc (spread) | final cyc (spread) |
|---|---|---|---|---|
| str | 4841 | 4841 (-0.0 %) | 1576 (13.0 %) | 1555 (15.1 %) -1.3 % |
| strfmt | 3314 | 3308 (-0.2 %) | 1182 (3.7 %) | 1067 (2.8 %) -9.7 % |
| strops | 2745 | 2657 (-3.2 %) | 864 (25.6 %) | 860 (14.4 %) -0.5 % |
| strscan | 1722 | 1704 (-1.1 %) | 601 (8.3 %) | 588 (20.4 %) -2.3 % |
| sprintfb | 3268 | 3259 (-0.3 %) | 1045 (10.7 %) | 1158 (10.8 %) +10.8 % |
| strcmp | 676 | 677 (+0.2 %) | 219 (7.7 %) | 219 (20.9 %) -0.0 % |
| render_span_kernel | 7256 | 7256 (+0.0 %) | 2014 (5.1 %) | 2032 (4.9 %) +0.9 % |
| gen_gc | 2627 | 2626 (-0.0 %) | 969 (5.9 %) | 943 (8.5 %) -2.7 % |

| AOT | base ins | final ins | base cyc (spread) | final cyc (spread) |
|---|---|---|---|---|
| str | 2849 | 2849 (-0.0 %) | 1042 (34.3 %) | 991 (13.3 %) -4.9 % |
| strfmt | 2362 | 2355 (-0.3 %) | 881 (12.9 %) | 889 (13.5 %) +1.0 % |
| strops | 2256 | 2168 (-3.9 %) | 765 (11.7 %) | 697 (12.4 %) -8.8 % |
| strscan | 1720 | 1701 (-1.1 %) | 581 (38.8 %) | 584 (25.7 %) +0.5 % |
| sprintfb | 2763 | 2754 (-0.3 %) | 931 (13.3 %) | 885 (14.4 %) -5.0 % |
| strcmp | 523 | 524 (+0.2 %) | 196 (10.2 %) | 193 (23.0 %) -1.3 % |
| render_span_kernel | 4249 | 4249 (-0.0 %) | 946 (4.0 %) | 956 (2.9 %) +1.1 % |
| gen_gc | 1807 | 1805 (-0.1 %) | 764 (10.0 %) | 778 (6.9 %) +1.9 % |

Every benchmark prints the same output with both builds, interpreter and AOT
(`scripts/benchout.sh`).

* Instructions: at or below the baseline on seven benchmarks (strops -3.2 %, strscan -1.1 %),
  +0.2 % on strcmp (5 instructions per iteration; the candidate is the kind check of its two
  `String#length` calls, not broken down further).
* Cycles: the machine ran at a load average of 8 to 40 during the whole session and the
  spread of five runs is as large as the differences, in both directions and differently for
  the interpreter and the baked code of the same benchmark. Two full runs
  (`logs/perf.final3`, above, and `perf.final3b`) disagree with each other by more than the
  builds differ: sprintfb interp +10.8 % then +4.2 % with its AOT at -5.0 % and -7.3 %,
  strfmt interp -9.7 % then -0.1 %. Run alone with more repetitions (interleaved; minimum /
  median, M cycles): sprintfb 1028 / 1065 -> 1031 / 1064 (11 runs), strfmt 1059 / 1098 ->
  1017 / 1070 (9 runs), strscan 543 / 602 -> 548 / 579 (15 runs). No cycle regression is
  visible; one under about 5 % could not have been seen here.
  One cycle difference was large enough to stand out of this noise earlier in the work, and
  was real: "Fewer instructions, more cycles" below.

Where the numbers come from, in the order the work went (`logs/ic.*`, instruction counts,
interp):

| step | str | strfmt | strops | strscan | sprintfb | strcmp | render_span_kernel | gen_gc |
|---|---|---|---|---|---|---|---|---|
| base | 4841 | 3314 | 2744 | 1722 | 3268 | 675 | 7256 | 2626 |
| string.rs + string_methods.rs converted as they stood (b92b160) | 4841 | 3322 | 2834 | 1872 | 3262 | 690 | 7256 | 2630 |
| + `split` scans a stack copy instead of re-borrowing per piece, sprintf.rs, encoding.rs | 4840 | 3322 | 2798 | 1671 | 3285 | 692 | 7256 | 2630 |
| + thin accessors `inline(always)`, `" "` recognized without decoding the pattern | 4841 | 3322 | 2680 | 1671 | 3286 | 679 | 7256 | 2630 |
| + `format` reads each String once | 4840 | 3322 | 2680 | 1671 | 3282 | 679 | 7256 | 2630 |
| + `enc_single` answers UTF-8 without the encoding table (ed1be64) | 4840 | 3307 | 2650 | 1671 | 3267 | 676 | 7256 | 2625 |
| + stringio, strscan, symbol_methods, the CRuby fixes (b767946) | 4841 | 3307 | 2650 | 1671 | 3265 | 676 | 7256 | 2625 |
| + a new String's buffer is built where it is allocated (1c403de) | 4841 | 3307 | 2656 | 1703 | 3259 | 676 | 7256 | 2625 |

So the conversion itself, before anything was tuned, cost +0.2 % (strfmt, gen_gc: one check in
`String#length` per iteration), +2.2 % (strcmp: `str_eq` out of line, 8 instructions per
element of `include?`), +3.3 % (strops) and +8.7 % (strscan: `split` re-borrowing the string
for every piece, about 19 instructions per piece). The last two were how the code was written,
not the design (recipe patterns 12 and 17). What remains of the design after that is the
`+0.2 %` class; the row "`enc_single`" is an unrelated saving on the same paths (the encoding
table lookup in `String#length`, `index`, `[]`), kept as a separate step so that it does not
pass for the design being free.

A difference under about 0.5 % is also what an unrelated change does to these numbers: a
build with a change confined to symbol.rs moved sprintfb by +0.5 % and gen_gc by +2.0 %, and
per-function counts (`callgrind`) put all of it outside symbol.rs (`hash_aset` no longer
inlined into the hash literal node; `sprintf::format` 14 instructions per call longer). With
16 codegen units the inlining decisions of ThinLTO depend on what else is in the crate, as the
pilot found with the allocator. The control, both trees built with
`CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1`:

| one codegen unit, M instructions | base interp | final interp | base AOT | final AOT |
|---|---|---|---|---|
| str | 4949 | 4949 (+0.0 %) | 2872 | 2872 (+0.0 %) |
| strfmt | 3332 | 3325 (-0.2 %) | 2370 | 2363 (-0.3 %) |
| strops | 2734 | 2670 (-2.3 %) | 2242 | 2179 (-2.8 %) |
| strscan | 1731 | 1705 (-1.5 %) | 1729 | 1702 (-1.6 %) |
| sprintfb | 3288 | 3267 (-0.6 %) | 2773 | 2751 (-0.8 %) |
| strcmp | 688 | 689 (+0.1 %) | 534 | 535 (+0.2 %) |
| render_span_kernel | 7247 | 7247 (+0.0 %) | 4239 | 4239 (+0.0 %) |
| gen_gc | 2617 | 2616 (-0.0 %) | 1798 | 1797 (-0.1 %) |

### Fewer instructions, more cycles: a store-forwarding stall

The tuned build ran `String#split` (benchmark strscan) in 3 % fewer instructions than the
baseline and 30 % more cycles: 705 M against 543 M, minimum of 15 interleaved runs, with the
slower build never under 675 M in any of about 50 runs. `perf annotate` put 41 % of the
samples of `Ctx::str_from_sbuf` on one `vmovups 0x10(%rsp),%xmm1`.

`c.str_new_enc(b, enc)` was `self.str_from_sbuf(SBuf::from_slice(b), enc)`. `SBuf` is three
words with the short-string bytes inline; `from_slice` was inlined into the caller, which
wrote a few bytes and the length byte into its stack slot and passed the slot to the
out-of-line `str_from_sbuf`; that reloaded the three words as wide loads to move them into
the object. A wide load that overlaps narrower stores still in the store buffer cannot be
forwarded and waits for them to retire (the likely mechanism: it was not confirmed with a
counter; the evidence is where the samples sit and that they are gone after the change). Now
`str_from_sbuf` is `inline(always)` and
`str_new_enc` takes the length from the slice and builds the buffer in the function that
allocates, as the context-free original did: 547 M cycles (minimum of 15, baseline 543 M in
the same run), 1703 M instructions (baseline 1722 M).

A by-value buffer handed to an out-of-line allocator is the natural way to write "build the
payload while the source is borrowed, then allocate" under the borrow rule (recipe step 2:
`c.str_from_sbuf(SBuf::from_slice(..), enc)`), so this is a cost of the design's idiom, and
the instruction count does not show it. Keep the constructor and the allocation in one
function (`str_new_enc`, `str_copy`, `str_sub` do), and measure cycles on whatever allocates
in a loop.

### Verification

* `make` and `make FEATURES=raw_stack` without warnings and `ruby tools/heap_audit.rb` exit 0
  at each of the seven commits (built at each one; `scripts/snap.sh`).
* The guard rails fire: `rstr(v)`, `str_enc(v)` and `str_new_enc(b"x", 0)` appended to
  string_methods.rs are three `E0659`; `let b = c.str_bytes(v); let x = c.str_new(b"y"); .. b ..`
  and the same with `expect_str(c, v)?` in the middle are two `E0502`.
* Differential tests against CRuby 4.0.2, `run_specs.rb --jobs 4 --timeout 15 --diff ruby`,
  TOTAL rows (PASS / FAIL / CRASH / WCRASH / TIMEO / REFTO), the two builds alternately, twice,
  at load averages of 14 to 27 (`logs/totals.final3`). All four runs give the same rows:

  | directory, pattern | base | final |
  |---|---|---|
  | t/hand `str*` (`string*`, `stringio*`, `strscan*` included) | 2350 / 195 / 0 / 0 / 0 / 0 | 2350 / 195 / 0 / 0 / 0 / 0 |
  | t/hand `sym*` (`symbol*` included) | 118 / 2 / 0 / 0 / 0 / 0 | 118 / 2 / 0 / 0 / 0 / 0 |
  | t/hand `sprintf*` | 42 / 20 / 0 / 0 / 0 / 0 | 42 / 20 / 0 / 0 / 0 / 0 |
  | t/hand `format*` | 73 / 0 / 0 / 0 / 0 / 0 | 73 / 0 / 0 / 0 / 0 / 0 |
  | t/hand `encoding*` | 72 / 5 / 0 / 0 / 0 / 0 | 72 / 5 / 0 / 0 / 0 / 0 |
  | t/hand `regexp*` | 490 / 1304 / 0 / 0 / 1 / 0 | 490 / 1304 / 0 / 0 / 1 / 0 |
  | t/method `string*` | 17080 / 31 / 0 / 0 / 0 / 0 | 17080 / 31 / 0 / 0 / 0 / 0 |
  | t/method `symbol*` | 1601 / 0 / 0 / 0 / 0 / 0 | 1601 / 0 / 0 / 0 / 0 / 0 |

  (An earlier pair of runs had `regexp*` FAIL 1301 for the baseline once and 1304 otherwise:
  the directory is shared and one timeout-sensitive file moves.)
* Per-file output (stdout, stderr, exit status; 60 s limit) of the 191 files `t/hand/`
  `{str,sym,sprintf,format,encoding,regexp}*.rb` and the 235 files `t/method/{string,symbol}*.rb`
  (`str*` covers `string*`, `stringio*`, `strscan*`; `sym*` covers `symbol*`): identical to the
  baseline's for the four commits that convert (d43563c, b92b160, ed1be64, f1ecd68); two runs of
  the baseline are identical to each other, so no file here is nondeterministic. From 1d4f2cb on,
  one line of one file differs, on purpose:
  `string_fixes_1018.rb` line 6, `s.split { s.prepend("xxx" * 100) }`, printed `:ok` and now
  prints `RuntimeError: string modified`, which is what CRuby prints for that line run alone
  (in the file CRuby 4.0.2 itself dies with SIGSEGV three lines earlier, at
  `"AB".force_encoding(Encoding::UTF_32LE).lines(sep)`, so the harness never compared it).
* The same for the 126 files `t/hand/str*.rb` under `SARUBY_GC_STRESS=1`: identical to the
  baseline under stress but for the same line, at every commit.
* `repro/alias.rb` and `repro/splitmod.rb`, 146 one-line programs about self-aliasing
  (`s << s`, `s.replace(s)`, `s.concat(s, s)`, `s[1, 2] = s`, ...), frozen and encoding checks,
  and callbacks (`to_str`, `to_int`, `to_s`, blocks, a Hash default proc) that clear, grow or
  replace the receiver, each run in its own process: the baseline differs from CRuby on 65 of
  them, the final build on 7.

### Bugs found

In the baseline, each with the shortest program that shows it; "fixed" means the final build
prints what CRuby prints.

* **Stale buffer after `to_str`** (a `&'static [u8]` read before the conversion, used after):
  `s = +"abc"; o = Object.new; o.define_singleton_method(:to_str) { s << "y" * 300; "c" }; s.chomp(o).size`
  gave 3 (bytes of a buffer that had moved), CRuby 303. Same in `chomp!` and in `split` with a
  pattern that needs `to_str`. Fixed: self is read after the conversion.
* **`StringIO#read(n)` with a `to_int` that replaces the string** read the old string
  (`io.read(o)` with `o.to_int` doing `io.string = "x"`: `"lo w"`, CRuby `"x"`). Fixed.
* **Uninitialized StringIO**: `Class.new(StringIO) { def initialize; end }.new.read`
  dereferenced nil as a String (SIGSEGV). Fixed: IOError "uninitialized stream", as CRuby.
* **Panics on a length read before a conversion**:
  `s = +"abc" * 50; o = Object.new; o.define_singleton_method(:to_int) { s.clear; 100 }` and then
  `s[o, 2] = "x"`, `s.byteslice(o, 5)`, `s.bytesplice(o, 5, "x")` each panicked (index out of
  range in Rust; not undefined behaviour, but the process dies). Fixed: IndexError / nil.
* **`each_byte` yielded bytes that were gone**: `s.each_byte { s.clear }` ran 150 times with
  0 for every byte after the first; `each_codepoint` iterated a list made up front. Fixed:
  both re-read the string at every step (1 iteration, as CRuby).
* **"string modified" was raised only by `sub!` / `gsub!` with a block.** CRuby also raises
  it from `split`, `scan`, `gsub`, `scrub`, `scrub!` with a block and from `gsub` / `sub!` with
  a Hash whose default proc changes the receiver. Fixed for changes of length (CRuby compares
  the buffer pointer and the length).
* **`sub` and `partition` read the receiver at the wrong moment**: CRuby's `sub` works on a
  copy taken before its arguments are converted, `partition` / `rpartition` convert the
  separator first. Fixed.

Not fixed, differences from CRuby that were there before and are not safety matters:
`tr` / `tr_s` / `squeeze` / `delete` read the receiver after converting their arguments
(CRuby dups first, like `sub`); `gsub!` / `sub!` with a block compare contents, so an in-place
change of the same length (`setbyte`) raises "string modified" where CRuby does not;
`bytesplice` accepts an index that needs `to_int`; `s.bytesplice(0, 2, s, 2, 3)` gives the
expected `"cdecdef"` where CRuby 4.0.2 gives `"ccdcdef"`.

Not a bug of saruby: `t/hand/string_fixes_1018.rb` crashes CRuby 4.0.2 (above), so everything
after its line 17 has never been compared.

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
   | `rstr(v)`, `str_bytes(v)` | `c.rstring(v)` / `c.rstring_mut(v)`, `c.str_bytes(v)`; `rstr(v).enc` is `c.str_enc(v)`; frozen check and write: `c.str_modify(v)?` |
   | `str_new_enc(b, e)`, `str_new_like(s, vec)`, `str_dup(s)`, `str_char_len(s)`, `char_offsets(s)`, `str_eq(a, b)`, `str_to_sym(s)`, `ary_of_strs(v, e)`, ... | the same names as methods of `c` (string.rs); add `use crate::heap::no_legacy_str::*;` so the free ones cannot come back |
   | `str_new_enc(str_bytes(s), enc)` (a copy), `str_new_like(s, b(s)[i..j].to_vec())` (a piece) | `c.str_copy(s)` / `c.str_copy_enc(s, enc)`, `c.str_sub(s, i..j)`: the bytes go straight into the new body (inline when short), no `Vec` in between; in general `c.str_from_sbuf(SBuf::from_slice(&c.str_bytes(s)[..]), enc)` |
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

Met on the String side (string.rs, string_methods.rs, sprintf.rs, encoding.rs, stringio.rs,
strscan.rs, symbol_methods.rs):

12. **Bytes needed across a call that takes the context.** A `&[u8]` cannot cross it, and three
    shapes cover what was there:
    * *Read after.* When the call is a conversion (`expect_str`, `num_to_long`: `to_str` /
      `to_int` run Ruby code), convert first and read the receiver after. This is not only what
      compiles: CRuby reads after the conversion too, and the old code read a stale buffer
      (`chomp`, `split`; see "Bugs found"). `let suf = expect_str(c, a[0])?; let (v, sb) =
      (c.str_bytes(slf), c.str_bytes(suf));`.
    * *Copy what is needed.* `let src = c.str_bytes(slf).to_vec();` before a loop that yields
      (`gsub`, `scan`, `scrub`: CRuby iterates over the contents it started with and raises
      "string modified" when the block changes the receiver). For a scan that allocates per
      piece and never calls out, `ByteSnap::new(c.str_bytes(slf))` copies to the stack when
      the string is short (128 bytes) and to a `Vec` otherwise (`String#split` fast path).
    * *Re-borrow per step*, when CRuby re-reads at every step:
      `while let Some(&x) = c.str_bytes(slf).get(i) { yield1(c, blk, Value::fix(x as i64))?; i += 1; }`
      (`each_byte`; `each_codepoint` decodes at the current offset the same way).
13. **A function that reads bytes in place and can raise.** `expand_repl(c, rep_bytes, ..)`
    took the context only to raise, which forbade passing it `c.str_bytes(rep)`. It now returns
    its errors by value (`Result<Vec<u8>, ReplErr>`) and the caller raises:
    `match expand_repl(c.str_bytes(r), ..) { Ok(x) => .., Err(e) => return Err(e.raise(c)) }`.
    The alternative, copying the replacement at every match, costs a malloc per match.
14. **Native payloads (`RData`) with several small fields** (`StrIO`, `RScan`). The old helper
    `fn sio(c, v) -> Result<&'static mut StrIO, Esc>` cannot become
    `-> Result<&StrIO, Esc>`: the error arm needs `&mut Ctx` while the Ok arm would keep it
    borrowed (a conditional return of a borrow, which today's borrow checker rejects). Read
    the state **by value** and write through one accessor:
    `fn sio(c: &mut Ctx, v: Value) -> Result<StrIO, Esc>` (`StrIO` is `Copy`; for `RScan`,
    whose match groups are a `Vec`, a `Copy` snapshot `Sc` of what the methods read) and
    `fn sio_mut(c: &mut Ctx, v: Value) -> &mut StrIO`. A method then reads `let d = readable(c, slf)?;`,
    computes, and ends with `sio_mut(c, slf).pos += n;`. Arguments are converted before the
    snapshot is taken (`io.read(n)` with a `to_int` that replaces the string read the old one).
15. **Module state.** Class values (`StringScanner`, its `Error`) and the table of Encoding
    objects are `Vm` fields (`c.vm().c_string_scanner`, `c.vm().enc_objs`). Plain data that is
    not a `Value` and is read by functions without a context (the default external encoding,
    lazily built conversion tables) is a safe static: `AtomicU8`, `OnceLock<[u8; N]>`,
    `[OnceLock<RevMap>; N]`.
16. **Boot-time `init()`.** No context exists while `builtins::boot` runs, and nothing else
    does either, so no payload borrow can be alive: `init(vm: &Vm)` allocates with
    `crate::gc::alloc` directly and sets up classes through functions of vm.rs
    (`define_class`, `class_set_alloc`, `const_set`). An `init` that runs later (strscan, at
    the first `require`) takes the context like any other code.
17. **Thin accessors are `#[inline(always)]`** (`str_enc`, `str_len`, `str_eq`, `str_cmp`,
    `str_hash`). After `if is_string(a) && is_string(b)` the kind check of `c.str_eq(a, b)` is
    dead code, but only if the accessor is inlined into that function; out of line,
    `Array#include?` paid 8 instructions per element for it (benchmark strcmp).

## What is still open

* The call sites below.
* Allocation behind the context's back (71 `gc::alloc` sites), until `gc::alloc` is reachable
  only through `Ctx::alloc`.
* `Ctx::frame() -> &'static mut Frame` and the raw `*const Block` / `*const MethodEntry` /
  `*const Cref` graph: the frame chain, a separate surface (safe_stack.md lists it too).
* cfunc arguments `&[Value]`, `rdata` payloads holding Values, `Vec<Value>` scratch buffers:
  rooting, roadmap section 2.
* String side: 238 call sites of the context-free functions of str_legacy.rs are left in
  unconverted modules (`str_new` 135, `str_new_frozen` 21, `str_new_enc` 19,
  `str_from_vec_enc` 19, ...; `heap_audit.rb` does not count them as a column). They go with
  their own modules, after which `no_legacy_str` folds into `no_legacy`. Converted String code still calls into
  unconverted modules while holding no borrow: `regexp::*` (`rregexp` / `rmatch` hand out
  `&'static mut`; `search_raw(re, c.str_bytes(s), ..)` is called under a String borrow and
  touches only the Regexp), `hash_aref` / `hash_get_opt`, `funcall`, `big_to_value`
  (allocates without the context). `symbol.rs` (the table) and `sbuf.rs` (the buffer) keep
  their `unsafe`.

## Remaining work per file

`ruby tools/heap_audit.rb --md` at 1c403de (the String side; regenerate after a merge). Columns:
call sites of each old accessor (`rstr` includes `str_bytes`, `rproc` includes `rmethod`,
`ary_*` are the context-free array functions of object.rs); `sum`; functions of the file that
return `&'static mut`; `static mut` declarations; `unsafe` occurrences (comments excluded).
Generated `src/node_*.rs` and `src/prism_ast.rs` are not counted; `node.def`, their source, is.
The context-free String functions of str_legacy.rs (`str_new_enc`, `str_enc`, ...) are not
among the columns; a file that imports `heap::no_legacy_str` is checked for them.

| file |    obj |   rary |   rstr |  rhash | rclass |  rproc |   rbig |  rdata | ivars_of |  ary_* |     vm |   sum | fn->&'static mut | static mut | unsafe | state |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| src/aot.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      1 |     1 |                0 |          1 |      4 |  |
| src/array.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/bignum.rs |      1 |      0 |      0 |      0 |      0 |      0 |      1 |      0 |      0 |      0 |      1 |     3 |                0 |          0 |      1 |  |
| src/builtins/array_methods.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/builtins/binding.rs |      0 |      0 |      1 |      0 |      0 |      1 |      0 |      1 |      0 |      3 |      9 |    15 |                1 |          0 |      9 |  |
| src/builtins/complex.rs |      1 |      0 |      4 |      0 |      1 |      0 |      0 |      2 |      0 |      8 |      4 |    20 |                0 |          0 |      0 |  |
| src/builtins/encoding.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
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
| src/builtins/sprintf.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/builtins/string_methods.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/builtins/stringio.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/builtins/strscan.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/builtins/structs.rs |      1 |      6 |      7 |      0 |      7 |      0 |      0 |      0 |      0 |     31 |      7 |    59 |                0 |          0 |      0 |  |
| src/builtins/symbol_methods.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/builtins/sys.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      6 |  |
| src/builtins/thread.rs |      0 |     10 |      1 |      0 |      4 |      0 |      0 |      6 |      0 |     29 |     25 |    75 |                4 |          8 |     40 |  |
| src/builtins/time.rs |      0 |      0 |      7 |      0 |      1 |      0 |      0 |      4 |      0 |      2 |     10 |    24 |                1 |          0 |      2 |  |
| src/compile.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      4 |  |
| src/fiber.rs |      0 |      1 |      0 |      0 |      1 |      1 |      0 |      1 |      0 |      2 |     10 |    16 |                1 |          5 |     21 |  |
| src/gc.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      2 |     2 |                1 |          3 |     13 |  |
| src/hash.rs |      9 |      2 |      2 |     25 |      0 |      0 |      2 |      0 |      0 |      7 |      7 |    54 |                1 |          1 |      4 |  |
| src/heap.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |     13 |  |
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
| src/str_legacy.rs |      3 |      0 |     11 |      0 |      0 |      0 |      0 |      0 |      0 |      2 |      5 |    21 |                0 |          0 |      0 |  |
| src/string.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/symbol.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                1 |          1 |      2 |  |
| src/value.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |     11 |  |
| src/vm.rs |     23 |      3 |      2 |      5 |     61 |     10 |      0 |      1 |      1 |      7 |     55 |   168 |                2 |          2 |     68 |  |
| node.def |     11 |      0 |      2 |      0 |      0 |      0 |      0 |      0 |      0 |      3 |      9 |    25 |                0 |          0 |     22 |  |
| total |    150 |    110 |    193 |     52 |    238 |     57 |     28 |     41 |      1 |    569 |    562 |  2001 |               27 |         41 |    558 |  |
