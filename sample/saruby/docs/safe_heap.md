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
* **Second wave** (hash, range, struct, enumerable, numeric: 11 more files, 1842 call sites left):
  the section of that name below.
* **Third wave** (rational, complex, time, random, kernel, proc, object, module, regexp,
  regexp_methods, symbol: 11 more files under `forbid(unsafe_code)`, 974 call sites left,
  `unsafe` 565 -> 485 overall, heap.rs 19 -> 33): the section of that name below. It adds one
  assumption, H5 (the frames below a running builtin are alive), for the frame-chain reads.
* **Fourth wave** (marshal, thread, io, misc, process, exception, json, file, extras, env,
  prelude, pp: 12 more files under `forbid(unsafe_code)`, 976 -> 498 call sites left, `unsafe`
  498 -> 376 overall, `static mut` 36 -> 20): the section of that name below. Every libc call
  of those files goes through a safe wrapper in `builtins/sys.rs` (8 -> 67 `unsafe`, one
  justification each); heap.rs gains no `unsafe` (`Global` and `cur_method_name` are safe code).
* **Method layer** (binding.rs, the boot sequence, enc_tables.rs): every file under
  `src/builtins/` but sys.rs (the OS interface) is now `forbid(unsafe_code)`; `unsafe` 375 -> 366.
  A safe `SBuf` was built and measured and is **not** merged (it costs time on short-string
  workloads); it is on branch `sbuf-safe-rs29`. Section "Method layer" below.

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

### Accessors added by the second wave (hash / range / numeric / data)

`src/heap.rs`, the block "hash / range / numeric / data" (6 more `unsafe`, 18 in the file).

* `c.hash_mut_and(v) -> (&mut RHash, Others)`, `c.data_mut_and::<T>(v) -> Option<(&mut T, Others)>`,
  `others.hash(w)`, `others.data::<U>(w)`: the construction of `ary_mut_and`, same argument
  (`Others` refuses the object being written; distinct objects own disjoint payloads).
* `c.range(v) -> (Value, Value, bool)` and `c.range_set(v, b, e, excl)` are ordinary `body` /
  `body_mut` matches. The fields come back by value, so a Range is never borrowed across a call;
  `range_set` writes a `Body::Range` over a `Body::Range` (the kind stays).
* `float_val(v) -> f64` is a free function without the token, like the header accessors: a
  Float body is written by `gc::alloc` only (the object is frozen at birth), no accessor hands
  out a reference into one, and the read ends inside the function. It checks the kind (it was
  `unreachable_unchecked`).
* `c.method_entry(klass, mid) -> Option<&MethodEntry>` dereferences what `vm::lookup_method`
  returned. Sound because an entry is a leaked `Box` (`vm::new_me`) that nothing frees; the
  borrow of `c` additionally keeps it from being held across Ruby code, which may change an
  entry's visibility in place (`Module#private` does). Identity is compared with
  `std::ptr::eq(me, c.vm().ids.kernel_hash)`, no dereference.
* `c.block_params(blk) -> Option<&Params>` dereferences a `*const Block`. This one rests on a
  new assumption, **H4: the `blk` a builtin receives is null or its caller's `Block`, alive for
  the call** (and iseqs are never freed). It is the assumption `yield_block(c, blk, ..)`, a safe
  function, already makes on every yield; the frame chain is the separate surface listed under
  "What is still open". A builtin that manufactures a `*const Block` from an integer
  (`enum_native::ublk`) is inside that same trust.

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
* **H4** (above): a builtin's `blk` argument.
* **H5** (third wave): the frames from `c.frame` down are alive while a builtin reads them; and
  the in-place writes to a caller frame or a method entry assume the evaluator holds no Rust
  reference to it across the call (section "Third wave").
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

`heap::no_legacy_hn` does the same for the context-free API of hash.rs, range.rs, numeric.rs and
bignum.rs (`hash_pairs`, `hash_len`, `range_parts`, `big_to_value`, `to_big`, `int_from_i64`,
`num_to_f64`, ...). It is a second list, imported by the modules of that wave only, so that
the modules converted earlier (and the string wave, in progress in parallel) do not have to
give up those names in the same commit; the two lists merge when every importer of
`no_legacy` is clean of both. The bodies of the context-free forms that unconverted modules
still call live in `src/object.rs` (hash) and `src/legacy.rs` (range, numeric).

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

## Second wave: hash, range, struct, enumerable, numeric (2026-11-21)

Commits `af4b634..dc28c5a` on top of `4a06252` (`base` below). Raw data, scripts and logs:
`~/ruby/src/trials/2026-11-21-saruby-safe-heap-hash-num/`.

Converted, each `#![forbid(unsafe_code)]` with `heap::no_legacy` and `heap::no_legacy_hn`:
`hash.rs`, `builtins/hash_methods.rs`, `range.rs`, `builtins/range_methods.rs`, `builtins/structs.rs`,
`builtins/enum_native.rs`, `builtins/enumerator.rs`, `numeric.rs`, `bignum.rs`,
`builtins/numeric_methods.rs`, `builtins/math.rs`. Not converted: `builtins/rational.rs`,
`builtins/complex.rs`, `builtins/time.rs`, `builtins/random.rs` (why: "What is still open").

| file | old-accessor call sites before -> after | `unsafe` before -> after |
|---|---|---|
| src/hash.rs | 54 -> 0 | 4 -> 0 |
| src/builtins/hash_methods.rs | 89 -> 0 | 2 -> 0 |
| src/range.rs | 4 -> 0 | 0 -> 0 |
| src/builtins/range_methods.rs | 38 -> 0 | 0 -> 0 |
| src/builtins/structs.rs | 59 -> 0 | 0 -> 0 |
| src/builtins/enum_native.rs | 82 -> 0 | 1 -> 0 |
| src/builtins/enumerator.rs | 64 -> 0 (and its `renum() -> &'static mut`) | 1 -> 0 |
| src/numeric.rs | 3 -> 0 | 0 -> 0 |
| src/bignum.rs | 3 -> 0 | 1 -> 0 |
| src/builtins/numeric_methods.rs | 76 -> 0 | 0 -> 0 |
| src/builtins/math.rs | 8 -> 0 | 0 -> 0 |
| src/builtins/rational.rs | 31 -> 31 (not converted) | 0 |
| src/builtins/complex.rs | 20 -> 20 (not converted) | 0 |
| src/builtins/time.rs | 24 -> 24 (not converted) | 2 |
| src/builtins/random.rs | 23 -> 23 (not converted) | 2 |

`heap_audit.rb` totals: call sites 2305 -> 1842, `unsafe` 570 -> 567 (heap.rs 12 -> 18: the six
blocks listed under "Accessors added by the second wave"), functions returning `&'static mut`
32 -> 31, `static mut` 48 -> 47 (the list of hashes being iterated is gone; the tool prints 48
because its pattern also matches the type `&'static mut crate::..` in fiber.rs).

Edits outside the converted files: `object.rs` (`RHash.iter_lev`, the context-free hash API,
`float_val` re-exported from heap.rs), new `legacy.rs` (context-free range / numeric API and
`f64_to_int`), `builtins/mod.rs` (the `init` calls pass and receive classes), `lib.rs` (`mod legacy`),
`fiber.rs` (a private `renum`), `array_methods.rs` (`each_overridden(c, ..)`, `is_arith_seq(c, ..)`),
`kernel.rs`, `sprintf.rs`, `random.rs` (path of `f64_to_int`), `pp.rs`, `time.rs`, `complex.rs`
(an import that became unused), `tools/heap_audit.rb` (the second list).

### Bugs the conversion exposed

Each is a place where the old code read a table through a reference that a callback could
invalidate, or kept state that the borrow rules forced into the open. One-line repros;
"before" is `base`.

| repro | CRuby | before | now |
|---|---|---|---|
| `h = (1..100).to_h { [_1, _1] }; n = 0; h.each { \|k, _\| h.delete(k); n += 1 }; p [n, h.size]` | `[100, 0]` | `[59, 41]`: `hash_delete` compacted the slots under the iteration | fixed (no compaction while `iter_lev != 0`) |
| `class K; def initialize(h) = @h = h; def hash = 7; def eql?(o) = (@h.clear if $go; false); end; h = {}; 12.times { h[K.new(h)] = 1 }; $go = true; p h[K.new(h)]` | `nil` | panic `index out of bounds` (hash.rs: the probe loop kept the index mask across `eql?`) | fixed (pattern 12) |
| `o = Object.new; def o.hash = raise("boom"); a = [o]; 2.times { begin; a.hash; p :no; rescue => e; p e.message; end }` | `"boom"` twice | `"boom"`, then `:no`: the recursion guard stayed set after the raise | fixed |
| `h = {"a" => 1}; begin; h.each { h.compare_by_identity }; rescue => e; p e.message; end; p h["a"]` | `"compare_by_identity during iteration"`, `1` | `"rehash during iteration"`, `nil`: the flag was set before the rehash failed | fixed |
| `h = {}; h.replace({}.compare_by_identity); p h.compare_by_identity?` | `true` | `false` | fixed |
| `S = Struct.new(:a); s = S.new(nil); s.a = s; p s.hash == s.hash` | `true` | the process dies (unbounded recursion) | fixed (recursion guard in `Struct#hash`) |
| `class R < Range; def each; yield 9; end; end; p R.new(1, 3).to_a` | `[9]` | `[1, 2, 3]` | not fixed (`Range#to_a` does not look for an overriding `each`); unrelated to borrowing |

Not reproduced but removed with the design: the old guard was a global `Vec<Value>` of hashes
being iterated, compared by address. An iteration abandoned in a fiber left its entry there, and
a hash allocated later at the same address would have refused new keys.

### Verification

* `make` and `make FEATURES=raw_stack` without warnings at each commit (the second
  configuration by `cargo check` for the last three), `ruby tools/heap_audit.rb` exit 0.
* Differential tests against CRuby, the task's patterns (`run_specs.rb --jobs 4 --timeout 15`),
  `base` and the new build alternately, twice. Cells are PASS/FAIL/CRASH/WCRASH/TIMEO/REFTO.

| pattern | base1 | new1 | base2 | new2 |
|---|---|---|---|---|
| hand hash* | 851/38/0/0/1/0 | 851/38/0/0/1/0 | 851/38/0/0/1/0 | 851/38/0/0/1/0 |
| hand range* | 350/122/0/0/0/0 | 350/122/0/0/0/0 | 350/122/0/0/0/0 | 350/122/0/0/0/0 |
| hand struct* | 229/5/0/0/0/0 | 229/5/0/0/0/0 | 229/5/0/0/0/0 | 229/5/0/0/0/0 |
| hand enum* | 735/34/0/0/0/0 | 735/34/0/0/0/0 | 735/34/0/0/0/0 | 735/34/0/0/0/0 |
| hand lazy* | 139/16/0/0/0/0 | 139/16/0/0/0/0 | 139/16/0/0/0/0 | 139/16/0/0/0/0 |
| hand set* | 178/42/0/0/0/0 | 178/42/0/0/0/0 | 178/42/0/0/0/0 | 178/42/0/0/0/0 |
| hand data* | 54/0/0/0/0/0 | 54/0/0/0/0/0 | 54/0/0/0/0/0 | 54/0/0/0/0/0 |
| hand comparable* | 34/0/0/0/0/0 | 34/0/0/0/0/0 | 34/0/0/0/0/0 | 34/0/0/0/0/0 |
| hand integer* | 88/3/0/0/0/0 | 88/3/0/0/0/0 | 88/3/0/0/0/0 | 88/3/0/0/0/0 |
| hand float* | 202/1/0/0/0/0 | 202/1/0/0/0/0 | 202/1/0/0/0/0 | 202/1/0/0/0/0 |
| hand numeric* | 329/42/0/0/0/0 | 329/42/0/0/0/0 | 329/42/0/0/0/0 | 329/42/0/0/0/0 |
| hand bignum* | 56/0/0/0/0/0 | 56/0/0/0/0/0 | 56/0/0/0/0/0 | 56/0/0/0/0/0 |
| hand rational* | 119/6/0/0/0/0 | 119/6/0/0/0/0 | 119/6/0/0/0/0 | 119/6/0/0/0/0 |
| hand complex* | 100/30/0/0/0/0 | 100/30/0/0/0/0 | 100/30/0/0/0/0 | 100/30/0/0/0/0 |
| hand math* | 24/3/0/0/0/0 | 24/3/0/0/0/0 | 24/3/0/0/0/0 | 24/3/0/0/0/0 |
| hand time* | 358/49/0/0/0/0 | 358/49/0/0/0/0 | 358/49/0/0/0/0 | 358/49/0/0/0/0 |
| hand random* | 134/34/0/0/0/0 | 134/34/0/0/0/0 | 134/34/0/0/0/0 | 134/34/0/0/0/0 |
| method hash* | 12452/0/0/0/0/0 | 12452/0/0/0/0/0 | 12452/0/0/0/0/0 | 12452/0/0/0/0/0 |
| method range* | 5952/0/0/0/0/0 | 5952/0/0/0/0/0 | 5952/0/0/0/0/0 | 5952/0/0/0/0/0 |
| method integer* | 14737/418/0/0/0/0 | 14737/418/0/0/0/0 | 14737/418/0/0/0/0 | 14737/418/0/0/0/0 |
| method float* | 5778/235/0/0/0/0 | 5778/235/0/0/0/0 | 5778/235/0/0/0/0 | 5778/235/0/0/0/0 |
| load average at start | 15.66 22.23 21.16 | 14.17 20.63 20.68 | 11.72 19.00 20.12 | 9.63 17.43 19.53 |
| all patterns | 42899/1078/0/0/1/0 | 42899/1078/0/0/1/0 | 42899/1078/0/0/1/0 | 42899/1078/0/0/1/0 |

* Per-file output (stdout, stderr, exit status; 60 s limit) of the 799 files those patterns
  select, new build against `base`: 793 identical, 0 different, 6 nondeterministic
  (they differ between two runs of `base` itself: object addresses, times, random numbers).
  Checked after each of the four groups of commits.
* The same under `SARUBY_GC_STRESS=1` for `hash*`, `struct*`, `enum*` (296 files): all 296 identical between `base` and the new build.
  One file (`hash_value_semantics.rb`, 25 s alone under stress on either build) hit the 60 s limit
  in one run while the machine's load average was above 12; the comparison runs such a file
  again alone before calling it a difference.
* The baseline was checked to be what it claims: the snapshot used for the comparisons and a
  separate build of `4a06252` give the same instruction counts on all 13 benchmarks, both modes.
  (The first benchmark run was thrown away: the snapshot of the new build could not bake, so its
  "AOT" rows were interpreter runs. `bench.sh` now takes build trees and compares outputs.)

### Cost

`bench.sh` (trials directory): per benchmark each build bakes once, then the two builds run
interleaved in each of 5 repetitions under `perf stat -e cycles:u,instructions:u`. Minimum, in
millions; spread is (max-min)/min. Load average during the run: 38 at the start, 15 to 33 afterwards (shared machine, so
read the cycles with their spread).

| interp | base ins | new ins | base cyc (spread) | new cyc (spread) | cyc |
|---|---|---|---|---|---|
| hash | 6797 | 5553 (-18.3 %) | 2321 (36.7 %) | 1958 (25.8 %) | -15.6 % |
| hashiter | 2452 | 2372 (-3.3 %) | 924 (36.6 %) | 659 (91.1 %) | -28.7 % |
| mapreduce | 4728 | 4594 (-2.8 %) | 2025 (8.0 %) | 1893 (20.3 %) | -6.5 % |
| methodchain | 4662 | 4470 (-4.1 %) | 1435 (22.9 %) | 1236 (22.7 %) | -13.9 % |
| rangeeach | 1391 | 1399 (+0.6 %) | 396 (60.7 %) | 388 (19.4 %) | -2.1 % |
| nesteddata | 986 | 999 (+1.3 %) | 319 (9.8 %) | 325 (11.6 %) | +1.8 % |
| structacc | 4400 | 4410 (+0.2 %) | 1126 (13.5 %) | 1154 (10.2 %) | +2.5 % |
| cmpsort | 3364 | 3368 (+0.1 %) | 1040 (32.4 %) | 976 (49.5 %) | -6.1 % |
| intdiv | 5464 | 5464 (-0.0 %) | 1894 (17.2 %) | 1851 (11.6 %) | -2.3 % |
| bignum | 687 | 689 (+0.3 %) | 291 (13.2 %) | 286 (18.3 %) | -1.8 % |
| mathfn | 4235 | 4238 (+0.1 %) | 1225 (9.8 %) | 1262 (9.8 %) | +3.0 % |
| floatcalc | 4984 | 4984 (+0.0 %) | 1441 (10.4 %) | 1437 (6.7 %) | -0.3 % |
| gc_wb | 1668 | 1668 (+0.0 %) | 577 (16.0 %) | 561 (18.0 %) | -2.8 % |

| aot | base ins | new ins | base cyc (spread) | new cyc (spread) | cyc |
|---|---|---|---|---|---|
| hash | 3692 | 2358 (-36.1 %) | 1251 (24.5 %) | 891 (15.4 %) | -28.8 % |
| hashiter | 1811 | 1731 (-4.4 %) | 500 (71.3 %) | 484 (78.4 %) | -3.2 % |
| mapreduce | 3692 | 3558 (-3.6 %) | 1007 (66.3 %) | 966 (72.1 %) | -4.0 % |
| methodchain | 3880 | 3688 (-4.9 %) | 1113 (35.1 %) | 1051 (31.8 %) | -5.5 % |
| rangeeach | 1022 | 1030 (+0.8 %) | 289 (12.8 %) | 293 (29.7 %) | +1.5 % |
| nesteddata | 714 | 730 (+2.4 %) | 252 (10.3 %) | 254 (9.9 %) | +1.0 % |
| structacc | 2486 | 2496 (+0.4 %) | 604 (14.2 %) | 621 (6.8 %) | +2.9 % |
| cmpsort | 2479 | 2484 (+0.2 %) | 775 (52.9 %) | 725 (61.5 %) | -6.5 % |
| intdiv | 1649 | 1649 (+0.0 %) | 357 (46.1 %) | 376 (13.7 %) | +5.3 % |
| bignum | 549 | 551 (+0.3 %) | 234 (11.5 %) | 231 (21.2 %) | -1.5 % |
| mathfn | 2471 | 2474 (+0.1 %) | 734 (36.6 %) | 752 (9.4 %) | +2.4 % |
| floatcalc | 1841 | 1841 (+0.0 %) | 408 (12.5 %) | 400 (12.2 %) | -2.0 % |
| gc_wb | 842 | 842 (+0.0 %) | 348 (18.1 %) | 356 (17.4 %) | +2.1 % |

Reading it, per benchmark:

* **hash (-18 % / -36 % instructions)**: `h[i & 4095] = i`. An immediate key is now found and
  stored under one borrow with a loop that compares keys only (`find_imm`); before, an indexed
  table went through the generic path (`key_hash`, `lookup_with_hash`, `key_eql`). Converted
  as it stood, it was +8 % / +11 %: six separate accessor calls per store (pattern 19).
* **hashiter (-3 % / -4 %)**: the block's parameter list is examined once per `each`, not per pair.
* **mapreduce, methodchain (-2.8 % to -4.9 %)**: `(a..b).map` keeps its result in a local. The generic
  `run_each` path, converted mechanically, was +2.5 % and +1.5 % (three checked accesses per element).
* **nesteddata (+1.3 % / +2.4 %)**: the kind checks themselves, 32 instructions per iteration for
  four `hash_aset` and two `hash_aref` on literals. This is the design's cost where nothing was restructured.
* **rangeeach (+0.6 % / +0.8 %), structacc, cmpsort, bignum, mathfn (within 0.4 %)**: no accessor
  was added on these paths (`Range#each` reads the range once per call). One instruction per
  iteration in rangeeach appeared with the numeric_methods commit, which does not touch
  range_methods.rs: inlining across modules, not checks.
* **intdiv, floatcalc, gc_wb**: unchanged.
* **Cycles**: the spreads are 7 to 91 % under this load. Only hash moves by more than its spread
  (-16 % / -29 %, the direction of its instruction count). For the rest the table shows no
  regression and cannot show more; it wants a rerun on a quiet machine.

The allocator accident of the pilot came back once, in the other direction: with the context-free
range / numeric functions appended to `object.rs`, `__rust_alloc` and `__rust_dealloc` were
separate symbols again (6.6 % + 4.1 % of nesteddata's instructions; nesteddata +5 %, gc_wb +1.3 %,
methodchain +0.2 %). The same functions in a module of their own (`legacy.rs`) give the
numbers above. The build still depends on a ThinLTO threshold for this; see "The allocator
accident".

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

## Third wave: rational, complex, time, random, kernel, proc, object, module, regexp, symbol (2026-11-21)

Commits `45694a5..866baed` on top of `5791694` (`base` below). Scripts, logs, repros and
results: `~/ruby/src/trials/2026-11-21-saruby-safe-heap-3/`.

### What was converted

| file | old-accessor call sites | `fn -> &'static mut` | `static mut` | `unsafe` | state |
|---|---|---|---|---|---|
| src/builtins/rational.rs | 31 -> 0 | 0 | 0 | 0 | `forbid(unsafe_code)` |
| src/builtins/complex.rs | 20 -> 0 | 0 | 0 | 0 | `forbid(unsafe_code)` |
| src/builtins/time.rs | 24 -> 0 | 1 -> 0 | 0 | 2 -> 0 (`localtime_r` moved to sys.rs) | `forbid(unsafe_code)` |
| src/builtins/random.rs | 23 -> 0 | 1 -> 0 | 1 -> 0 (`DEFAULT` is `Vm.default_random`) | 2 -> 0 | `forbid(unsafe_code)` |
| src/builtins/kernel.rs | 108 -> 0 | 0 | 1 -> 0 (`STDOUT_BUF`) | 14 -> 0 | `forbid(unsafe_code)` |
| src/builtins/proc.rs | 102 -> 0 | 0 | 0 | 30 -> 0 | `forbid(unsafe_code)` |
| src/builtins/object.rs | 211 -> 0 | 0 | 1 -> 0 (`ALIAS_NAMES` is `Vm.alias_names`) | 44 -> 0 | `forbid(unsafe_code)` |
| src/builtins/module.rs | 8 -> 0 | 0 | 0 | 1 -> 0 | `forbid(unsafe_code)` |
| src/regexp.rs | 17 -> 0 | 2 -> 0 (`rregexp`, `rmatch`) | 0 | 0 | `forbid(unsafe_code)` |
| src/builtins/regexp_methods.rs | 34 -> 0 | 1 -> 0 | 0 | 0 | `forbid(unsafe_code)` |
| src/symbol.rs | 0 | 1 -> 0 | 1 -> 0 | 2 -> 0 | `forbid(unsafe_code)` |
| src/builtins/sys.rs | 0 | 0 | 0 | 6 -> 8 | `local_tm` (the libc call time.rs made) |
| src/heap.rs | 0 | 0 | 0 | 19 -> 33 | the third-wave block below |
| all files (`heap_audit.rb` total) | 1548 -> 974 | 26 -> 20 | 41 -> 36 | 565 -> 485 | |

Edits outside these files, each the smallest that keeps the callers compiling: `vm.rs`
(fields `default_random` and `alias_names`), `builtins/mod.rs` (the `init` calls receive what
they define; the `ARGF` autoload line moved there from `kernel::init`; `alias_names`
initialized), `legacy.rs` (`recursion_enter` / `recursion_leave`, `time_from_ns` for
File::Stat), `io.rs` (`STDOUT_SYNC` is an `AtomicBool`: one `unsafe` fewer), `numeric_methods.rs`,
`array_methods.rs` (`rand_below(c, ..)`), `hash_methods.rs` (`proc_arity(c, ..)`),
`string_methods.rs`, `strscan.rs`, `symbol_methods.rs`, `marshal.rs`, `process.rs`, `file.rs`,
`encoding.rs`, `rt.rs` (the regexp / rational / time helpers take `c`), `node.def` (the `$1`
node: `nth_match(c, ..)`).

### Accessors added (heap.rs, block "third wave")

14 more `unsafe`, each one line with its justification; all but `with_iseq` read or write the
frame chain or VM-global data, not object payloads.

* `c.body_of(v) -> &Body`: `body` made public (Object#dup dispatches on the whole body). No new `unsafe`.
* Frame chain, read: `c.caller_frame()`, `c.cur_frame()`, `c.prev_frame(f)`. **H5: the frames
  from `c.frame` down are alive while the builtin reading them runs** (each lives on a caller's
  Rust stack until that caller returns, and the builtin returns first). The references borrow
  `c`, so they cannot be held across Ruby code. `Ctx::frame() -> &'static mut Frame` (vm.rs)
  still exists; converted modules do not call it.
* Frame chain, write: `c.set_caller_visi(v)` (`private` with no arguments), `c.set_caller_cref(cr)`
  (`using`). Place writes through the raw pointer, as the code they replace did; no reference
  to the frame is made. Whether the evaluator holds a `&mut Frame` of the caller across the
  call is the frame surface's question, not answered here.
* `c.me_ref(me) -> Option<&MethodEntry>`, `c.cref_ref(cr)`, `c.iseq_ref(is)`: entries are
  leaked Boxes (vm::new_me), crefs are leaked by vm::new_cref (and listed in `vm.crefs`),
  iseqs are never freed. Borrowing `c` keeps them from being held across Ruby code, which
  changes an entry's visibility in place.
* `c.me_set_visi(me, v)`: that in-place change, as a place write; `&mut self` excludes every
  `me_ref` borrow. The same caveat as the frame writes: a reference the evaluator might hold
  to the entry being run is outside what the borrow checker sees.
* `c.cref_set_refinements(cr, refs)` (a cref just made by `new_cref`), `c.refinements_on()` /
  `c.set_refinements_on()` (`static mut REFINEMENTS_ON` of vm.rs, read and written by value on
  the one interpreter thread).
* `c.with_iseq(is, |c, is| ..)`: an `&Iseq` beside `&mut Ctx`. Sound because nothing makes a
  `&mut Iseq` (they are built, leaked and then only read, `Cell` fields included), so the
  shared reference conflicts with nothing `&mut Ctx` reaches; the closure scopes it. `Class#new`
  calls `invoke_iseq_simple(c, me, is, ..)` through it (an `&Iseq` from `iseq_ref` would borrow
  `c` and conflict).
* `c.block_copy(blk) -> Option<Block>`: the builtin's Block by value (H4).

Not in heap.rs, and safe code: `symbol.rs`. Names are leaked byte strings (`&'static [u8]` is
literally true) in an append-only table of `OnceLock` chunks, so `sym_bytes` reads without a
lock; `intern` answers from a direct-mapped cache of `AtomicU32` slots (id + 1, verified
against the name on a hit, Release / Acquire so a name is visible before its id) and takes a
`Mutex` around the `HashMap` only on a miss; `lookup` takes the Mutex. Costs: "Cost" below.

### Recipe patterns met in this wave

21. **Arithmetic helpers that allocate** (`iadd`, `imul`, `idiv`, `rat_raw`, `ex_*`): give them
    `c: &mut Ctx`, read-only ones (`icmp`, `isneg`, `istr`, `rat_parts`) `c: &Ctx`, and name every
    intermediate: `isub(a, imul(idiv(a, b), b))` becomes `let q = idiv(c, a, b); let m = imul(c, q, b); isub(c, a, m)`.
    Small helpers that bundle a nesting keep the call sites short (`secs_of(c, w)` in time.rs).
22. **A `Copy` snapshot is not a place.** `rt(c, t).mode = UTC` (pattern 15's `rt` returns
    `RTime` by value) compiles without a warning and writes a temporary. It did, in the first
    build of time.rs (`Time.at(0, in: "+09:00:01")` printed `+0000`); the differential tests
    caught it. Writes go through `rt_mut(c, t)` or a setter (`set_mode`); grep for
    `snapshot(..).field =` after converting.
23. **A payload whose Values sit in a `Box`** (`Body::Data(Box::new(RRational { num, den }))`,
    `Body::Proc(Box::new(ProcData { blk, .. }))`) is built before the allocation, so during the
    collection the Values are only in malloc'ed memory. Use `c.alloc_with(k, move || Body::..)`:
    the closure (with the Values) stays on the stack until the slot exists.
24. **Collect, then allocate** when the data is borrowed from something that is not a payload
    (`Proc#parameters` walks an iseq: rows of Symbols are collected under the borrow and turned
    into Arrays after it).
25. **Method entries**: read fields through `me_ref`; to define a copy elsewhere, `me_copy`
    (fields by value) then `Box::into_raw`. Method tables are walked by collecting
    `(name, visibility)` first (`mtbl_entries`), never while pushing to a result Array.
26. **Boot `init` returns what it defines** and takes `vmr: &Vm`; `vm` is a banned name, and a
    binding named `vm` is E0659 too. `(vm.c_regexp, vm.c_match_data) = regexp_methods::init(vm)`;
    object::init returns its five method identities.
27. **Global state that is not a Value**: a `thread_local!` only if every user runs on the
    interpreter thread. `STDOUT_BUF` qualifies (only Ruby code writes it); the symbol map does
    not: `symbol::init` runs on the main thread and Ruby on the thread lib.rs spawns, so a
    thread-local map interned the predefined names twice and every id was off (tried, printed
    `exception` for `p 1`). Otherwise `Mutex`, atomics, `OnceLock`.
28. **Qualified legacy paths escape the ban**: `crate::hash::hash_new()`,
    `crate::numeric::float_new(f)`, `crate::bignum::int_from_i128(..)` compile in a converted
    file, because the glob ban only shadows unqualified names. `heap_audit.rb` does reject them
    (it also matches `::name(`), so run it before trusting a clean build: it caught
    `crate::bignum::big_to_value` in random.rs; kernel.rs had six more, found by grep first.

### Bugs found

"base" is `5791694`. Each line was run alone (`repro/bugs.rb`, `repro/run.sh`, output in
`results/repro.txt`); "now" prints what CRuby 4.0 prints.

| repro | base | now (= CRuby) |
|---|---|---|
| `abort(1)` | SIGSEGV (a Fixnum read as a String) | TypeError "no implicit conversion of Integer into String" |
| `autoload(1, "x")` | SIGSEGV | TypeError "1 is not a symbol nor a string" |
| `autoload(:X, 1)` / `autoload(:X, "")` | accepted | TypeError / ArgumentError "empty feature name" |
| `Module.new { refine(String) { import_methods 1 } }` | SIGSEGV | TypeError "wrong argument type Integer (expected Module)" |
| `class String; def upcase = :x; end; p :y.upcase` | panic `:x is not a heap object` (exit status 0) | `:Y` (Symbol#upcase calls the String builtin, not the redefinable method) |
| `"ab" =~ /(?<x>a)/; p Regexp.last_match(:x)` | `nil` (the name read with `as_fix`) | `"a"` |
| `SARUBY_GC_68 identical to base under stress, 5 nondeterministic as above, 4 different: `regexp_dup_named_group`, `regexp_engine_1018`, `regexp_many_groups`, `regexp_ruby_layer_1018`, the MatchData#to_a bug; with the final build their stress output equals their normal output, which equals base's normal output.=1`, `p(/\A(?<x>.)(?<x>.)\k<x>\z/.match("abb").to_a)` | `["b", "b", "b"]` | `["abb", "a", "b"]` |

The last one is a rooting bug: `MatchData#to_a` built the group Strings into a Rust `Vec` and
then allocated the Array, so a collection in between freed the Strings and the slots were
reused. Four files of `t/hand/regexp*` show it under stress. The conversion pushes each String
into the Array as it is made. Latent, not reproduced: `clone_class` read the source class
through a `'static` alias while allocating the new one; `Hash.ruby2_keywords_hash` iterated a
`Vec` of pairs across `hash_aset` (which can call `#hash`); Rational / Complex / Time / Proc
bodies held their Values only in a `Box` during allocation (pattern 23).

A panic in the interpreter thread exits with status 0 (lib.rs joins the thread and returns),
so a harness that looks at the status does not see panics; the scan below greps the output.

### Verification

* `make` and `make FEATURES=raw_stack` without warnings at each of the ten commits, and
  `ruby tools/heap_audit.rb` exit 0 (`scripts/snap.sh`).
* Differential tests against CRuby 4.0.2, `run_specs.rb --jobs 4 --timeout 15 --diff ruby`,
  the task's patterns in `t/hand` and `t/method`, base and ab6aa6c (everything but the intern
  cache) alternately, twice, then 866baed once (`results/totals.*`). All rows are equal in all
  runs (PASS / FAIL / CRASH / WCRASH / TIMEO / REFTO); the table shows the first four:

  | directory, pattern | base run 1 | final run 1 | base run 2 | final run 2 |
  |---|---|---|---|---|
  | hand rational* | 119 / 6 / 0 / 0 / 0 / 0 | 119 / 6 / 0 / 0 / 0 / 0 | 119 / 6 / 0 / 0 / 0 / 0 | 119 / 6 / 0 / 0 / 0 / 0 |
  | method rational* | 2099 / 147 / 0 / 0 / 0 / 0 | 2099 / 147 / 0 / 0 / 0 / 0 | 2099 / 147 / 0 / 0 / 0 / 0 | 2099 / 147 / 0 / 0 / 0 / 0 |
  | hand complex* | 118 / 12 / 0 / 0 / 0 / 0 | 118 / 12 / 0 / 0 / 0 / 0 | 118 / 12 / 0 / 0 / 0 / 0 | 118 / 12 / 0 / 0 / 0 / 0 |
  | method complex* | 857 / 0 / 0 / 0 / 0 / 0 | 857 / 0 / 0 / 0 / 0 / 0 | 857 / 0 / 0 / 0 / 0 / 0 | 857 / 0 / 0 / 0 / 0 / 0 |
  | hand time* | 358 / 49 / 0 / 0 / 0 / 0 | 358 / 49 / 0 / 0 / 0 / 0 | 358 / 49 / 0 / 0 / 0 / 0 | 358 / 49 / 0 / 0 / 0 / 0 |
  | hand random* | 134 / 34 / 0 / 0 / 0 / 0 | 134 / 34 / 0 / 0 / 0 / 0 | 134 / 34 / 0 / 0 / 0 / 0 | 134 / 34 / 0 / 0 / 0 / 0 |
  | hand kernel* | 216 / 57 / 0 / 0 / 0 / 0 | 216 / 57 / 0 / 0 / 0 / 0 | 216 / 57 / 0 / 0 / 0 / 0 | 216 / 57 / 0 / 0 / 0 / 0 |
  | hand proc* | 230 / 87 / 0 / 0 / 1 / 0 | 230 / 87 / 0 / 0 / 1 / 0 | 230 / 87 / 0 / 0 / 1 / 0 | 230 / 87 / 0 / 0 / 1 / 0 |
  | hand lambda* | 47 / 1 / 0 / 0 / 0 / 0 | 47 / 1 / 0 / 0 / 0 / 0 | 47 / 1 / 0 / 0 / 0 / 0 | 47 / 1 / 0 / 0 / 0 / 0 |
  | hand object* | 82 / 38 / 0 / 0 / 0 / 0 | 82 / 38 / 0 / 0 / 0 / 0 | 82 / 38 / 0 / 0 / 0 / 0 | 82 / 38 / 0 / 0 / 0 / 0 |
  | hand module* | 206 / 89 / 0 / 0 / 0 / 0 | 206 / 89 / 0 / 0 / 0 / 0 | 206 / 89 / 0 / 0 / 0 / 0 | 206 / 89 / 0 / 0 / 0 / 0 |
  | hand class* | 387 / 56 / 0 / 0 / 0 / 0 | 387 / 56 / 0 / 0 / 0 / 0 | 387 / 56 / 0 / 0 / 0 / 0 | 387 / 56 / 0 / 0 / 0 / 0 |
  | hand method* | 235 / 46 / 0 / 0 / 0 / 0 | 235 / 46 / 0 / 0 / 0 / 0 | 235 / 46 / 0 / 0 / 0 / 0 | 235 / 46 / 0 / 0 / 0 / 0 |
  | hand define* | 169 / 18 / 0 / 0 / 0 / 0 | 169 / 18 / 0 / 0 / 0 / 0 | 169 / 18 / 0 / 0 / 0 / 0 | 169 / 18 / 0 / 0 / 0 / 0 |
  | hand regexp* | 490 / 1304 / 0 / 0 / 1 / 0 | 490 / 1304 / 0 / 0 / 1 / 0 | 490 / 1304 / 0 / 0 / 1 / 0 | 490 / 1304 / 0 / 0 / 1 / 0 |
  | hand match* | 26 / 0 / 0 / 0 / 0 / 0 | 26 / 0 / 0 / 0 / 0 / 0 | 26 / 0 / 0 / 0 / 0 / 0 | 26 / 0 / 0 / 0 / 0 / 0 |
  | hand sym* | 118 / 2 / 0 / 0 / 0 / 0 | 118 / 2 / 0 / 0 / 0 / 0 | 118 / 2 / 0 / 0 / 0 / 0 | 118 / 2 / 0 / 0 / 0 / 0 |
  | method sym* | 1601 / 0 / 0 / 0 / 0 / 0 | 1601 / 0 / 0 / 0 / 0 / 0 | 1601 / 0 / 0 / 0 / 0 / 0 | 1601 / 0 / 0 / 0 / 0 / 0 |
  | hand eval* | 151 / 76 / 0 / 0 / 0 / 0 | 151 / 76 / 0 / 0 / 0 / 0 | 151 / 76 / 0 / 0 / 0 / 0 | 151 / 76 / 0 / 0 / 0 / 0 |
  | hand binding* | 77 / 43 / 0 / 0 / 0 / 0 | 77 / 43 / 0 / 0 / 0 / 0 | 77 / 43 / 0 / 0 / 0 / 0 | 77 / 43 / 0 / 0 / 0 / 0 |
  | load average at start | 12.49 17.95 17.21 | 19.22 18.49 17.41 | 13.48 16.97 16.96 | 13.91 16.50 16.79 |

* Per-file output (stdout, stderr, exit status; 60 s limit) over the same patterns (360 files):
  342 identical to base (with c9 and with 866baed); 18 differ, all of them files whose output differs between two runs of base itself (15 equal after masking hex addresses and long numbers; the other 3: child pids, a tmp path, unseeded random draws); 0 other differences.
* Under `SARUBY_GC_STRESS=1`, `proc*`, `object*`, `regexp*` (77 files):
  68 identical to base under stress, 5 nondeterministic as above, 4 different: `regexp_dup_named_group`, `regexp_engine_1018`, `regexp_many_groups`, `regexp_ruby_layer_1018`, the MatchData#to_a bug; with the final build their stress output equals their normal output, which equals base's normal output.
* Panic scan: every file of `t/hand`, `t/spec`, `t/method` (2949) run with each build, output
  grepped for `panicked at`: base 1 (`sdi_inline_sends.rb`, the Symbol#upcase line above),
  final 0 (c9 and 866baed).

### Cost

`scripts/perf.sh` (trials directory): per benchmark each build bakes once, then the builds run
interleaved in each of 5 repetitions under `perf stat -e cycles:u,instructions:u`; minimum, in
millions, and spread (max-min)/min. `final` = 866baed. Load average 6.9 at the start, 2.5 at
the end (`results/perf.final.4`; the shared development machine, not sp4: ssh was out of scope).

| interp | base ins | final ins | base cyc (spread) | final cyc (spread) | cyc |
|---|---|---|---|---|---|
| object | 3573.0 | 3573.0 (0.0 %) | 961.7 (5.4 %) | 981.0 (4.0 %) | +2.0 % |
| method_call | 14829.7 | 14829.7 (0.0 %) | 3933.3 (0.8 %) | 3853.1 (1.3 %) | -2.0 % |
| send | 11999.4 | 11999.3 (0.0 %) | 3322.1 (6.4 %) | 3109.7 (7.9 %) | -6.4 % |
| closures | 3790.8 | 3790.7 (0.0 %) | 1061.9 (2.1 %) | 1074.0 (1.0 %) | +1.1 % |
| block | 2894.3 | 2894.3 (0.0 %) | 843.9 (7.4 %) | 799.7 (4.9 %) | -5.2 % |
| mathfn | 4221.9 | 4221.8 (0.0 %) | 1130.7 (1.9 %) | 1120.5 (1.3 %) | -0.9 % |
| bignum | 689.1 | 689.1 (0.0 %) | 237.4 (10.3 %) | 236.1 (1.8 %) | -0.5 % |
| exception | 1421.5 | 1417.0 (-0.3 %) | 441.5 (12.4 %) | 432.1 (9.3 %) | -2.1 % |
| casewhen | 5715.0 | 5715.0 (0.0 %) | 1555.5 (4.0 %) | 1498.8 (7.0 %) | -3.6 % |
| strfmt | 3331.1 | 3331.1 (0.0 %) | 1003.3 (4.1 %) | 960.8 (3.2 %) | -4.2 % |
| symbench | 4895.1 | 4441.3 (-9.3 %) | 1276.5 (1.5 %) | 1136.9 (2.6 %) | -10.9 % |

| AOT | base ins | final ins | base cyc (spread) | final cyc (spread) | cyc |
|---|---|---|---|---|---|
| object | 1425.6 | 1425.5 (0.0 %) | 378.4 (0.9 %) | 375.7 (5.8 %) | -0.7 % |
| method_call | 3630.1 | 3630.1 (0.0 %) | 831.4 (1.3 %) | 836.7 (1.1 %) | +0.6 % |
| send | 2460.1 | 2460.1 (0.0 %) | 542.7 (3.5 %) | 541.9 (0.8 %) | -0.1 % |
| closures | 2515.0 | 2514.9 (0.0 %) | 736.1 (2.3 %) | 744.5 (2.1 %) | +1.1 % |
| block | 1725.3 | 1725.2 (0.0 %) | 420.3 (0.6 %) | 423.5 (6.1 %) | +0.8 % |
| mathfn | 2257.9 | 2257.9 (0.0 %) | 630.2 (1.2 %) | 630.6 (2.1 %) | +0.1 % |
| bignum | 553.6 | 553.6 (0.0 %) | 188.7 (8.3 %) | 190.8 (6.6 %) | +1.1 % |
| exception | 628.9 | 624.4 (-0.7 %) | 194.8 (33.2 %) | 195.3 (4.8 %) | +0.3 % |
| casewhen | 1131.8 | 1131.8 (0.0 %) | 262.4 (3.3 %) | 262.5 (1.8 %) | +0.0 % |
| strfmt | 2266.0 | 2265.9 (0.0 %) | 742.0 (6.5 %) | 756.7 (2.0 %) | +2.0 % |
| symbench | 3051.2 | 2598.3 (-14.8 %) | 791.0 (0.8 %) | 636.8 (1.0 %) | -19.5 % |

Reading it, per benchmark:

* **Instructions**: within 0.1 % of base on the ten benchmarks the brief names, both modes;
  the conversions are not on their hot paths except `Class#new` (object: `c_new` now reads the
  entry with `me_ref` and calls `invoke_iseq_simple` inside `with_iseq`, same count) and the
  symbol table. exception -0.3 % / -0.7 % and symbench -9 % / -15 %: the intern cache (below).
* **Cycles**: no benchmark is slower by more than its spread; the largest increases (object
  interp +2.0 %, strfmt AOT +2.0 %) are inside them. send / block interp -6 % / -5 % on the same
  instruction count are code placement, not this change (block interp -6.3 % and -5.8 % in two
  earlier runs; send -1.4 % in one).
* **symbench** (`scripts/symbench.rb`: `String#to_sym`, `Symbol#to_s`, `Symbol#length` in a loop).
  The symbol table was the one place where the safe design had a run-time price of its own.
  Measured in steps (AOT instructions, M): base 3051; a `Mutex` around a table that also
  holds the names (the String side's attempt) +8 %; names lock-free and the Mutex only on the
  map 3169-3228 (+4 to +6 %: about 25 instructions for lock + unlock per `to_sym`, plus two
  `OnceLock` loads per `sym_bytes`); the lock-free intern cache 2597 (-15 %, -20 % cycles). The
  saving is the cache's FxHash and single compare replacing SipHash and the `HashMap` probe on
  a hit, not something the safety work bought; without it the table costs the +4 to +6 %.
* **A cost that showed in cycles only.** With `intern`'s fast path `#[inline]` (it has 600-odd
  callers) bignum AOT ran +5.2 % to +7.9 % cycles in three runs (11 repetitions each) on an
  identical instruction count; c6 and c8 (before the cache) did not. `big_to_value`, unchanged
  and byte-identical in the disassembly apart from data addresses, went from 9 % to 15 % of the
  cycle samples. A smaller cache (16 KB instead of 256 KB) did not change it; `intern` out of
  line did (190.9 against base 188.8 M). Not traced further; the working assumption is code
  placement in libsaruby.so, as the pilot's allocator accident.

## Fourth wave: marshal, thread, io, misc, process, exception, json, file, extras, env, prelude, pp (2026-11-21)

Commits `580e813..52842b4` on top of `e9ece96` (`base` below). Scripts, logs, repros and
results: `~/ruby/src/trials/2026-11-21-saruby-safe-heap-4/`. The work ran as four parallel
branches (thread/process, io/file, marshal/json, the rest) on top of two shared commits (sys.rs,
heap.rs `Global`), merged here; the merge commits record the one-line conflicts.

### What was converted

`heap_audit.rb` columns, before (`e9ece96`) -> after. `unsafe` "before" is at `e9ece96`; the
first commit (sys.rs wrappers) already moves the libc `unsafe` of these files into sys.rs.

| file | old-accessor call sites | `fn -> &'static mut` | `static mut` | `unsafe` | state |
|---|---|---|---|---|---|
| src/builtins/marshal.rs | 80 -> 0 | 0 | 0 | 0 | `forbid(unsafe_code)` |
| src/builtins/thread.rs | 75 -> 0 | 4 -> 0 (`th`, `thr`, `lk`, `qu`) | 8 -> 0 (LIST, MAIN, CURRENT, C_KILL, C_FATAL: `Global`; ABORT_ALL, REPORT_ALL, HELD: atomics) | 40 -> 0 | `forbid(unsafe_code)` |
| src/builtins/io.rs | 58 -> 0 | 3 -> 0 (`rio`, `get_io`, `readable`) | 0 | 31 -> 0 | `forbid(unsafe_code)` |
| src/builtins/misc.rs | 53 -> 0 | 0 | 2 -> 0 (ENC_OBJS removed with dead code, FINALIZERS: `Global`) | 6 -> 0 | `forbid(unsafe_code)` |
| src/builtins/process.rs | 43 -> 0 | 0 | 3 -> 0 (HANDLERS: `Global`; LAST_STATUS, DIE_SIGNAL: atomics) | 54 -> 0 | `forbid(unsafe_code)` |
| src/builtins/exception.rs | 43 -> 0 | 0 | 0 | 0 | `forbid(unsafe_code)` |
| src/builtins/json.rs | 38 -> 0 | 0 | 2 -> 0 (M_JSON, C_STATE: `Global`) | 8 -> 0 | `forbid(unsafe_code)` |
| src/builtins/file.rs | 37 -> 0 | 0 (`stat_md` returned a `&'static` without `mut`, not counted; gone too) | 1 -> 0 (CHDIR_DEPTH: atomic) | 25 -> 0 | `forbid(unsafe_code)` |
| src/builtins/extras.rs | 26 -> 0 | 0 | 0 | 13 -> 0 | `forbid(unsafe_code)` |
| src/builtins/env.rs | 10 -> 0 | 0 | 0 | 4 -> 0 | `forbid(unsafe_code)` |
| src/builtins/prelude.rs | 3 -> 0 | 0 | 0 | 0 | `forbid(unsafe_code)` |
| src/builtins/pp.rs | 2 -> 0 | 0 | 0 | 0 | `forbid(unsafe_code)` |
| src/str_legacy.rs | 21 -> 11 | 0 | 0 | 0 | 7 functions lost their last caller and were removed |
| src/legacy.rs | 11 -> 10 | 0 | 0 | 0 | `time_from_ns` removed |
| src/builtins/sys.rs | 0 | 0 | 0 | 8 -> 67 | every libc call of the files above, one safe wrapper each |
| src/heap.rs | 0 | 0 | 0 | 33 -> 33 | `Global`, `cur_method_name` (safe code) |
| all files (`heap_audit.rb` total) | 976 -> 498 | 20 -> 13 | 36 -> 20 | 498 -> 376 | |

Edits outside these files, each the smallest that keeps the callers compiling: `builtins/mod.rs`
(the boot `init`s receive the Vm; `lazy_const` moved there from prelude.rs, which re-exports it,
plus `lazy_consts`), `lib.rs` (`setup_load_path(&mut c)`, `setup_data(&mut c, ..)`,
`flush_all(c)`), `random.rs` and `kernel.rs` (one call each: `errno_class(c.vm(), ..)`,
`last_status(c)`).

### sys.rs: the OS interface

`builtins/sys.rs` was a list of `pub extern "C"` declarations called with `unsafe` from six
modules. The declarations are now private (`mod ffi`) and each call has a safe wrapper that
takes Rust types, so the pointers it passes are valid for the call by construction:

* buffers as slices (`read(fd, &mut [u8])`, `write`, `pread`, `pwrite`), `pipe2(&mut [c_int; 2])`,
  `poll(&mut [PollFd], ms)`, out-parameters as `&mut` (`waitpid`, `clock_gettime`, `getrlimit`,
  `times`);
* paths as `&CStr` (`open`, `chmod`, `chown`, `truncate`, `mkfifo`, `chdir`, `utimensat_cwd`);
* `execvp(&CStr, &[CString])` builds the NULL-terminated array itself;
* `signal(sig, SigAction::{Default, Ignore, Catch(extern "C" fn(c_int))})` instead of a `usize`
  handler address;
* what was open-coded in the callers: `fd_metadata(fd)` (`File::from_raw_fd` in a
  `ManuallyDrop`, so the borrowed fd is never closed), `read_dir_raw(&CStr)` (opendir / readdir /
  closedir, each entry copied out before the next readdir), `block_all_signals()`,
  `set_itimer_real()`, the libm functions of Math (`erf`, `erfc`, `cbrt`, `tgamma`, `lgamma_r`).

Each `unsafe` (67) has a one-line `SAFETY` comment; blocks of wrappers of one shape share a
block comment that states the obligation once. What remains is the OS interface itself, which
the wrappers cannot discharge and say so: `fork` (the child either execs at once or runs the
interpreter thread alone), `setenv` / `unsetenv` / `clearenv` (race with `getenv` in another
thread; only the interpreter thread touches the environment), `signal` (the handler's own
contract: `process::on_signal` only sets an atomic), `_exit`.

### Module globals: `heap::Global`

A `static mut X: Value` was read and written with `unsafe` and kept alive by pushing it on
`Vm.pinned` once. `heap::Global` replaces it without `unsafe`: the Value lives in a slot of
`Vm.pinned` and the static holds only the slot index (an `AtomicUsize`). `g.get(c)` /
`g.set(c, v)` go through `c.vm()` / `c.vm_mut()`, `g.set_vm(vm, v)` is the boot form. Since the
collector visits `Vm.pinned` in place, a moving collector would also update the value, which
the old statics did not allow. Plain data statics became atomics (`AtomicBool`, `AtomicI32`,
`AtomicU64` for `LAST_STATUS` with pid and status packed). Cost: an index load and a bounds
check per read; `thread::active(c)` (two reads and an Array length) is on the IO read path and
`sleep`, not on any benchmark's hot path.

### Recipe patterns met in this wave

29. **`static mut` holding a Value** -> `static G: Global = Global::new();` set at boot with
    `G.set_vm(vm, v)` or lazily on first use (`misc::finalizers(c)` makes its Array the first
    time `define_finalizer` runs, so boot allocates nothing for it).
30. **A native payload with a buffer** (`RIo`: `Vec<u8>` read and write buffers, a path). Not
    `Copy`, so pattern 14's snapshot does not apply. The checks run first in a function that
    returns `Result<(), Esc>` (`check_open`, `readable_check`), the accessor after it returns the
    `&mut RIo` (`get_io(c, v)?` = check, then `io_mut(c, v)`). After anything that runs Ruby code
    or lets another thread run (`to_int`, `thread::wait_readable`), the stream is checked again:
    another thread may have closed it (a bug in base, below).
31. **A data representation another module reads by name.** exception.rs used to keep each Errno
    class's message in an ivar `__errmsg` (an allocation per class, which a context-free
    `errno_class` could not make in a converted file); it now takes the text from the `ERRNOS`
    table by number. io.rs, converted in parallel, had copied the old `ivar_get(k, "__errmsg")`
    into its new `raise_errno_at`: it compiled, and would have printed "Unknown error" for every
    errno. Found while merging, by grepping for the ivar name. Rule: when a conversion drops a
    field, ivar or table, grep the whole tree for its name; the compiler only sees Rust names.
32. **Iteration order that depends on symbol ids.** `global_variables` iterated an `FxHashMap`
    keyed by Symbol; the merged build interns Symbols in another order (which change moved it was
    not traced: it is a property of the hash, not of a change) and `$stderr` came before `$stdin`
    in one test. Now the built-in names come first in a fixed order
    and the rest sorted. Any output built from a Symbol-keyed hash map has this property.
33. **Boot-time constant Strings without a context** (`RUBY_VERSION`, ...): a small helper that
    allocates through `crate::gc::alloc` and freezes (pattern 16 of the String side): io.rs
    `boot_frozen_str` and misc.rs `boot_str`, the same function twice (the two were written in
    parallel; one can go). Lazy `init`s (`env::init`, `json::init`, `file::init_stat`, `thread::init_sync`,
    `exception::define_all_errnos`, `misc::rbconfig_init`) run at `require` and take `c`.
34. **Walking the heap** (`ObjectSpace.each_object`, a dead thread's locks): `gc::each_object`
    does not allocate, so its closure may use `c` (`c.body_of(v)`, `c.data_mut::<RLock>(v)`); a
    result is filtered into a `Vec<Value>` before anything allocates and becomes an Array in one
    `c.ary_from_vec` (the pending body roots it).
35. **Hash iteration that calls out** (Marshal.dump of a Hash, JSON generate, ENV write-back,
    WeakMap#each, pp): slot by slot (`c.hash_entry_at(h, i)`), with `hash_iter_enter` /
    `hash_iter_leave` where CRuby forbids adding keys meanwhile (`marshal::each_pair`); never
    from a `Vec` of pairs taken up front (pattern 14).

### Bugs found

"base" is `e9ece96`. Each line run alone (`repro/{lead,io_file,marshal_json}.rb`,
`repro/run.sh`, output in `results/repro.txt`); "now" prints what CRuby 4.0 prints in every row.

| repro | base | now (= CRuby) |
|---|---|---|
| `r, w = IO.pipe; w.close; r.read(5, 123)` | SIGSEGV (an Integer read as a String) | TypeError |
| `r.read(5, "x".freeze)` at EOF | nil | FrozenError |
| `IO.read(path, -1.5)` | "negative length -9007199254740989 given" (a Float read as a Fixnum) | "negative length -1 given" |
| a `to_int` that closes the stream during `r.read(o)` | nil (read through a stale reference) | "closed stream" |
| `r.read(-1)` on a closed stream | "closed stream" | "negative length -1 given" |
| `File.realpath` / `Dir.pwd` with a latin-1 byte in the name | bytes replaced by U+FFFD (`path_arg` used `from_utf8_unchecked`: undefined behaviour on such bytes) | bytes kept |
| `File.readlines(dir)`, `File.foreach(dir)` | `[]`, nil | Errno::EISDIR |
| `IO.readlines(path, chomp: true)` under `SARUBY_GC_STRESS=1` | TypeError (the options Hash was collected) | correct |
| `JSON.load(src, proc)` | proc called on values only, result ignored | called on keys and values, its result used (json 3.1 `on_load`) |
| `JSON.load("  ")`, `JSON.load("", nil, allow_blank: false)` | nil | JSON::ParserError |
| `marshal_dump` / `to_json` that adds a key to the Hash being dumped | dumped silently | "can't add a new key into hash during iteration" |
| `marshal_dump` that clears its Hash and runs `GC.start` | dumps a freed String (79 bytes) | 18 bytes |
| `SystemCallError.new("x", 2)`, `SystemCallError.new(2)` | a SystemCallError | an Errno::ENOENT |
| `SystemCallError.new("x", 9999).message` | "unknown error - x" | "Unknown error 9999 - x" |
| `SystemCallError.new("message").errno` | the module `Errno` (found as `Object::Errno`) | nil |
| `SystemExit.new.message`, `SystemExit.new(3).message` | "exit" | "SystemExit" |
| `RuntimeError.new("a\nb").inspect` | `#<RuntimeError: "a\nb">` | `#<RuntimeError:"a\nb">` |

Closed by construction, not reproduced: `thread::run_pass` iterated a Rust copy of the thread
list while fibers ran (a thread killed before it started left the list and nothing rooted it);
`spawn_spec` kept env / option pairs in a `Vec` across `to_str` / `to_path` callbacks; Marshal.dump
dumped a Hash from a `Vec` of pairs across `marshal_dump`; the JSON parser held a `&'static [u8]`
into the source String while running callbacks.

Not fixed (differences from CRuby that were there before, outside the safety work):
`__callee__` of an aliased method returns the original name; `ObjectSpace::WeakMap#inspect`
inspects its keys; `Exception#detailed_message` / `full_message` formats for some arguments
(`t/spec/exception_000.rb` lines 5 to 15); `Errno` messages of `Dir.mkdir` say `rb_dir_s_mkdir`
where CRuby says `dir_s_mkdir`.

### Verification

* `make` and `make FEATURES=raw_stack` without warnings and `ruby tools/heap_audit.rb` exit 0 at
  every commit (each branch built and audited per commit; merges rebuilt; `scripts/snap.sh`).
* Per-file output (stdout, stderr, exit status; 60 s limit) of the 244 files of `t/hand`,
  `t/method`, `t/spec` whose names match the wave's patterns (marshal thread mutex queue condvar
  monitor io file dir stat process signal fork spawn system trap exception raise rescue ensure
  retry throw catch json env pp kernel sleep at_exit print puts gets pipe popen select argf
  pathname errno error objectspace object_space gc weak finaliz rbconfig math filetest warning),
  base run twice: 227
  identical; 15 differ between the two base runs (13 of them equal to base after masking hex
  addresses, numbers of 4+ digits and /tmp paths: `scripts/ndmask.sh`; the other 2,
  `exception_fixes_1017.rb` and `spec/exception_000.rb`, differ besides in the lines of the
  exception.rs fixes above, which now print CRuby's output); 2 differ on purpose:
  `file_read_dir_eisdir.rb` (EISDIR) and `json_object_to_json.rb` line 21 (`on_load`), both now
  CRuby's output.
* The same under `SARUBY_GC_STRESS=1` for marshal thread mutex queue io file dir process signal
  exception raise json env pp objectspace gc weak finaliz (175 files): 159 identical; 11
  nondeterministic (9 equal after masking, the same 2 exception files); 5 different: the same 2
  intended changes, `io_class_methods.rb` and `io_universal_newline_read.rb` (the `chomp:`
  rooting fix: their stress output now equals their normal output, the second up to the 60 s
  limit it hits in both builds), and `thread_rep_reap_1120.rb` (base needs 57 s alone under
  stress and timed out under load; final 44 s, the same output as without stress).
* Panic scan, every file of `t/hand`, `t/spec`, `t/method` (2949), output grepped for
  `panicked at`: base 0, final 0.

### Cost

`scripts/perf.sh` (trials directory): per benchmark each build bakes once, then the builds run
interleaved in each of 7 repetitions under `perf stat -e cycles:u,instructions:u`; minimum, in
millions, and spread (max-min)/min. `final` = 52842b4. Load average 4.6 at the start (18 and 24
over 5 and 15 minutes: the machine had just come off a load of 40), 4.8 at the end
(`results/perf.final.1`). object, exception, strfmt, str are rubyharness benchmarks; io_write
(a File written with `write` / `print` / `<<` and `$stdout.write`), marshal (dump + load of a
nested structure) and json (generate + parse) are in the trial's `scripts/`, since rubyharness
has none of that name.

| interp | base ins | final ins | base cyc (spread) | final cyc (spread) | cyc |
|---|---|---|---|---|---|
| object | 3509.2 | 3509.3 (0.0 %) | 935.5 (18.6 %) | 939.8 (7.3 %) | +0.5 % |
| exception | 1419.0 | 1421.0 (+0.1 %) | 451.8 (19.1 %) | 442.7 (4.1 %) | -2.0 % |
| strfmt | 3267.3 | 3267.4 (0.0 %) | 926.1 (5.7 %) | 990.5 (5.4 %) | +7.0 % |
| str | 4711.2 | 4711.2 (0.0 %) | 1226.8 (2.8 %) | 1225.2 (9.3 %) | -0.1 % |
| io_write | 3158.1 | 2874.7 (-9.0 %) | 911.1 (9.1 %) | 762.9 (3.3 %) | -16.3 % |
| marshal | 8826.1 | 8696.1 (-1.5 %) | 2736.1 (4.3 %) | 2680.6 (2.0 %) | -2.0 % |
| json | 9600.3 | 8618.9 (-10.2 %) | 2919.9 (91.6 %) | 2574.4 (91.1 %) | -11.8 % |

| AOT | base ins | final ins | base cyc (spread) | final cyc (spread) | cyc |
|---|---|---|---|---|---|
| object | 1337.7 | 1337.7 (0.0 %) | 346.8 (2.3 %) | 337.6 (4.2 %) | -2.7 % |
| exception | 626.4 | 628.5 (+0.3 %) | 199.2 (4.6 %) | 180.8 (11.8 %) | -9.2 % |
| strfmt | 2216.1 | 2216.2 (0.0 %) | 706.3 (1.5 %) | 729.0 (2.9 %) | +3.2 % |
| str | 2575.4 | 2575.4 (0.0 %) | 718.7 (3.2 %) | 695.9 (3.9 %) | -3.2 % |
| io_write | 2622.8 | 2340.1 (-10.8 %) | 721.8 (6.2 %) | 637.3 (5.2 %) | -11.7 % |
| marshal | 8821.0 | 8694.6 (-1.4 %) | 2780.0 (4.9 %) | 2677.2 (9.4 %) | -3.7 % |
| json | 9597.0 | 8610.9 (-10.3 %) | 2898.4 (3.9 %) | 2579.8 (4.2 %) | -11.0 % |

Every benchmark prints the same output with both builds. Reading it, per benchmark:

* **io_write (-9 % / -11 % instructions)**: `IO#write` / `print` / `<<` look the IO up once and
  write a String's bytes from the borrow, without copying them into a `Vec` first (the old code
  copied them).
  Cycles follow (-16 % / -12 %, beyond the spreads).
* **json (-10 %)**: the generator writes each String's bytes under one borrow instead of copying
  them, and no longer copies the indent / newline settings per value. Cycles follow in AOT
  (spread 4 %); the interpreter's spread (91 %) says one repetition of each was disturbed, the
  minimum is still the better estimate.
* **marshal (-1.5 %)**: not broken down (dump now reads each body once by value and walks Hashes
  slot by slot; load copies the source bytes once); cycles -2 % / -4 %, inside the spreads.
* **object, str (equal instructions)**: nothing of this wave is on their paths. Cycles within
  their spreads.
* **exception (+2.0 M, +0.1 % / +0.3 % instructions)**: callgrind on a 2-iteration run
  (`results/callgrind.exception.diff`): +12.5 instructions per raise, none of them in a
  function of the converted files. `core::ptr::drop_glue::<Body>` is a separate function in the
  final build (inlined into `gc::drop_body` in base) and `fiber::on_free` costs 8 more per
  call; both are unchanged code, so this is inlining across codegen units moving, the effect
  recorded for the pilot and the String side. Cycles: lower in both modes, inside the spreads.
* **strfmt (equal instructions, more cycles)**: interp +7.0 % in the run above, then +0.6 % and
  +0.9 % alone (11 and 15 repetitions, `results/perf.strfmt.{2,3}`): not reproducible. AOT
  +3.2 %, +3.9 %, +2.2 % in the three runs, spreads 1.5 to 8 %: consistent in sign, at the size
  of the spread. A `cycles:u` profile of both AOT builds has the same functions with the same
  shares (`rt::srb_dstr_append` 46 %, memmove 16 to 17 %, the baked code 13 to 14 %,
  `str_legacy::str_new_enc` 8 to 9 %): rt.rs and the baked code are untouched, `str_new_enc`
  is unchanged (str_legacy.rs only lost other functions). Not resolved; with nothing of this
  wave on the path, the candidates are code placement in `libsaruby.so` (as the third wave's
  bignum case) or the load. A quiet machine (sp4) would decide it; here cycles cannot.

## Method layer: binding, boot, enc_tables; the String buffer (2026-11-21)

Commits `85a975f`, `61270f2`, `0059037` on top of `322b70f` (`base`); the SBuf experiment is `a7a1f7e` on branch `sbuf-safe-rs29`. Scripts, numbers, the SBuf variants:
`~/ruby/src/trials/2026-11-21-saruby-unsafe-method-layer/`.

### What is `forbid(unsafe_code)` now

Every file under `src/builtins/` except `sys.rs` (the OS interface, fourth wave), and outside it
array.rs, avec.rs, bignum.rs, envvals.rs, hash.rs, inline_builtin.rs, inline_iter.rs, numeric.rs,
range.rs, regexp.rs, string.rs, symbol.rs (and the other files `heap_audit.rb` marks).
`builtins/mod.rs` itself only declares the modules; it cannot carry the attribute, which would
cover sys.rs.

| file | old-accessor call sites | `fn -> &'static mut` | `unsafe` | state |
|---|---|---|---|---|
| src/builtins/binding.rs | 14 -> 0 | 1 -> 0 (`rbind`) | 8 -> 0 | `forbid(unsafe_code)` |
| src/builtins/mod.rs | 26 -> 0 | 0 | 2 -> 0 | module list only |
| src/builtins/boot.rs (new: the boot sequence, `dup_data`, `alloc_data`) | 0 | 0 | 0 | `forbid(unsafe_code)` |
| src/builtins/enc_tables.rs (generated; generator too) | 0 | 0 | 0 | `forbid(unsafe_code)` |
| src/legacy.rs | 10 -> 14 | 0 | 0 | three boot helpers (below) |
| src/heap.rs | 0 | 0 | 33 -> 34 | `boot_vm` |
| all files | 485 -> 449 | 11 -> 10 | 375 -> 366 | |

* **binding.rs.** The iseq parent chain is walked with `c.iseq_ref`, the caller frame read with
  `c.caller_frame()` / `c.cur_frame()` (H5), the Binding payload through `c.data` / `c.data_mut`;
  a Binding is allocated with `alloc_with` (its Values stay in the closure on the stack until
  the slot exists: pattern 23). Env slots: `Ctx::env_up` / `env_get` / `env_set`, safe code in
  heap.rs on `body` / `body_mut`. `run_eval_iseq_env` still builds the eval frame itself and
  sets `c.frame` (a raw pointer assignment, safe); that is the frame surface, not heap access.
* **Boot.** `blank_vm` filled the class fields of a `MaybeUninit<Vm>` with nil words and
  `ptr::write` the rest; a field added after `main_obj` and not listed there was left
  uninitialized without a diagnostic. It is now a struct literal (a missing field is E0063).
  The VM is installed by `heap::boot_vm(vm, |vmr| ..)`: the one `unsafe` of boot, guarded by an
  atomic so that it runs once, before any `Ctx` exists. boot receives `&mut Vm` instead of calling
  `vm()`; the boot-time code it calls (`define_class`, `const_set`, ..) still uses `vm()`
  internally, which aliases that `&mut Vm` as it did before (the `vm()` item of "What is still
  open"). Class setup without a vm.rs setter (metaclass link, subclass list, autoload entries)
  is three helpers in legacy.rs (`boot_metaclass`, `boot_subclass`, `boot_autoload`): boot has no
  context, so they stay context-free, as pattern 16 says.

### What is left outside the method layer

`heap_audit.rb` after these commits, `unsafe` per file, by category:

| category | files (`unsafe`) |
|---|---|
| OS interface | builtins/sys.rs 67, kalloc.rs 12 (the allocator) |
| fiber switch | fiber_os.rs 22, fiber.rs 1 |
| loader / native code | aot.rs 11, prism.rs 1 (the parser FFI), compile.rs 7 (node tree built from raw node pointers) |
| heap accessor base | heap.rs 34, value.rs 11, gc.rs 16, object.rs 7 (the legacy accessors), lib.rs 2 (`Token::mint`, `BOOTED`) |
| call path / frames | vm.rs 80, rt.rs 64, node.def 16, stack_raw.rs 5 |
| String buffer | sbuf.rs 10 (below) |

### The String buffer: safe forms measured, not merged

`SBuf` (sbuf.rs) is three words: 23 bytes inline with the length in the last byte, else a
Vec-compatible heap block, with the tag in the top bit of the capacity; `RString` is `SBuf` +
`enc` (32 bytes), `Body` 40. Three safe forms, each `forbid(unsafe_code)`, an enum whose
inline-length type's unused values are the tag (as avec.rs):

* `boxvec`: `Inline(Len, [u8; 22]) | Heap(Box<Vec<u8>>)`, 24 bytes, a drop-in; one more pointer
  and one more malloc per long string.
* `vec15`: `Inline(Len, [u8; 15]) | Heap(Vec<u8>)`, 24 bytes, a drop-in (the largest inline
  array that keeps 24 bytes); strings of 16 to 23 bytes go to malloc.
* `rs29` (branch `sbuf-safe-rs29`): `Inline { len, aux, [u8; 29] } | Heap { aux, Vec<u8> }`,
  32 bytes, the encoding in `aux`, so `RString` stays 32 bytes and `Body` 40, with **29** bytes
  inline. `RString`'s fields become private (`enc()`, `set_enc()`, `set_bytes()` keeps the
  encoding, `Deref` to SBuf), which touches 22 files (node.def one line). `rs29b` adds a one-byte
  fast path to the heap append (the first build called memcpy for `s << "x"`).

M instructions (min of 5), interleaved runs, `perf stat -e cycles:u,instructions:u`, load average
9 to 13 at the start, 5 at the end (`results/perf.r2.md` has every row with cycles and spread;
`results/perf.r1.md` the first run with `boxvec`):

| bench | mode | base | boxvec | vec15 | rs29b |
|---|---|---|---|---|---|
| str | interp | 4649 | +18.1 % | +8.5 % | +4.6 % |
| str | AOT | 2600 | +29.5 % | +13.4 % | +6.0 % |
| strfmt | AOT | 1921 | +10.0 % | +24.7 % | +0.5 % |
| strscan | interp | 1614 | +14.3 % | +3.5 % | +9.4 % |
| strops | AOT | 1630 | +7.4 % | +0.7 % | +3.3 % |
| CRuby string_concat binary_concat_embedded | AOT | 1453 | +7.5 % | +21.3 % | -0.2 % |
| CRuby string_concat interpolation_same_heap | AOT | 553 | +14.7 % | +2.4 % | +6.5 % |
| CRuby string_split to_words_10 | AOT | 1583 | +20.2 % | +4.4 % | +7.9 % |
| CRuby string_rpartition rpartition_10 | AOT | 1250 | +24.3 % | +3.2 % | +5.1 % |
| CRuby string_dup uplus | AOT | 753 | +11.4 % | -0.5 % | +5.7 % |
| gcchurn, gc_wb, nesteddata, hash | both | | -1.4 to +1.8 % | -0.6 to 0 % | -0.3 to +1.5 % |
| optcarrot, per frame (N=5) | AOT | 84.72 | | 84.72 | 84.73 |

Cycles (min of 5; spreads 1 to 15 % on most rows, worse on some): rs29b is slower than base on
str (+5.1 % interp, +11.5 % AOT), interpolation_same_heap (+49 % / +62 %, spreads 1 % / 6 %),
upcase_10 / downcase_10 / swapcase_10 AOT (+19 to +25 %), rpartition_10 (+14 % / +18 %),
slice string_short (+13 %), dup (+20 % AOT), and faster where its 29 inline bytes keep a string
out of malloc: strfmt (-12 % / -25 %), binary_concat_embedded (-27 % / -40 %). optcarrot: 25.01 M
against 25.08 M cycles per frame (spread 2.6 %), the same. The cycle differences of several
other rows are inside their spread or code placement:

* **A placement effect, measured.** vec15 ran `String#include?` / `index` on a 4 KB haystack in
  48 % fewer cycles than base with the same instruction count (3 repetitions each, 333 to 347 M
  against 639 to 645 M), and boxvec did not. `perf annotate`: all of it is the byte loop of
  `find_bytes`, identical code, at `0x490ff0..0x491001` in base (it straddles a 64-byte boundary)
  and at `0x392680` in vec15. Not the buffer layout; such rows are read on instructions.

Where rs29b's instructions go (str, AOT): `extend_from_slice` on the enum is a match on a tag
byte that the compiler keeps in memory, with `Vec::push`'s own capacity check after the explicit
one, and two more callee-saved registers in `str_append_same_enc`; on allocation, the 29-byte
array is zeroed and the encoding is written through a second match (`RString::new`). The cost
is the shape of safe code around an enum, not bounds checks; some of it can be tuned further
(the first rs29 build was +9.8 % on str interp, rs29b +4.6 %).

**Recommendation.** Keep the unsafe `SBuf` on master for now: `sbuf.rs` is 186 lines whose 10
`unsafe` are `from_raw_parts` over a buffer it alone owns, and every String method above it is
already safe code. If the 29-byte inline buffer is wanted for its own sake (strfmt, string
building), take rs29 and tune its append and constructor paths first; boxvec and vec15 are not
worth merging (boxvec costs most; vec15 moves 16-to-23-byte strings to malloc for nothing).
Correctness of all three: the per-file outputs of `t/hand` `str*`, `sym*`, `encoding*`,
`sprintf*`, `format*` and `t/method` `str*`, `sym*` (415 files) equal base's; rs29b also under
`SARUBY_GC_STRESS=1` (400 files).

### Verification

* `make`, `make FEATURES=raw_stack`, `make GC=bitmap` without warnings and `heap_audit.rb`
  exit 0 at each commit (`scripts/snap.sh` builds all three).
* Per-file output (stdout, stderr, status; 60 s) of every file of `t/hand`, `t/spec`,
  `t/method` (2991), base twice and the final build once: 2924 identical to base, 64 identical
  after masking addresses and long numbers (base against itself: the same 64, plus 2 timing
  files), 3 different (`io_fd_numbers_1121`, `io_wait_forms`, `realpath_magic_link_1021`): an
  fd number one higher, because that snapshot directory had acquired a `preload_store/` from
  the benchmark runs (aot.rs maps it at start, one more memfd); without it the three print
  what base prints.
* The binding / eval / instance_eval / module_eval / class_eval / kernel / proc files after
  the binding.rs commit alone, and `str*` / `sym*` / `encoding*` / `sprintf*` / `format*` after
  each SBuf variant: identical to base (modulo addresses).
* `SARUBY_GC_STRESS=1` on `t/hand` + `t/method` + `t/spec` `str*`, `binding*`, `eval*`
  (400 files): 398 identical to base under stress, 2 after masking addresses.
* Panic scan (`panicked at` in the output) over the same 2991 files: base 0, final 0.


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
   | `hash_new()`, `hash_len(h)`, `hash_pairs(h)`, `hash_entry_at(h, i)`, `hash_clear(h)`, `hash_dup(h)` | the same names as methods of `c` (hash.rs) |
   | `range_parts(r)`, `range_new(b, e, x)` | `c.range_parts(r)` (nil, nil, false for a non-Range), `c.range(r)` (must be a Range), `c.range_new(b, e, x)` |
   | `rbig(v)`, `to_big(v)`, `big_to_value(b)`, `int_from_i64(i)`, `int_from_i128(i)`, `num_to_f64(v)`, `float_new(d)` | `c.big(v)`, `c.to_big(v)`, `c.big_to_value(b)`, ..., `c.float_new(d)`; `float_val(v)` stays a free function (by value) |
   | `lookup_method(k, m)` + `unsafe { (*me).kind }` | `c.method_entry(k, m)` -> `Option<&MethodEntry>`; identity with `std::ptr::eq` |
   | `unsafe { &*blk }` to look at the block's parameters | `c.block_params(blk)` |
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

Patterns met in the second wave (hash, range, struct, enumerable, numeric):

12. **Lookup that may call out** (`eql?` on a key). Scan the table under one shared borrow up
    to the first key that has to be asked, return what to ask as plain Values, drop the borrow,
    call, then **re-validate** before trusting any index (`RHash::probe` and
    `lookup_with_hash` in hash.rs): the callback may have emptied or rebuilt the table. A
    changed shape restarts the scan. Keys that cannot call out (immediates) are served under a
    single borrow (`find_imm`), which is also what makes them cheap: see 19.
13. **Iterating a table while calling a block**: by slot number, re-reading the slot at each
    step, `while let Some((k, v)) = c.hash_entry_at(h, i) { if k != UNDEF { .. f(c, k, v) .. } i += 1; }`,
    inside `c.hash_iter_enter(h)` / `c.hash_iter_leave(h)`. The iteration level is a field of
    the table (`RHash.iter_lev`, CRuby's `iter_lev`), not a global list; while it is non-zero a
    new key and a rehash raise, and **the slots are not compacted** (a compaction renumbers
    them under the iteration: that was a bug, see below).
14. **Filling one table from another when hashing a key runs Ruby code**: slot by slot from the
    source (`copy_entries` in hash_methods.rs), never from a `Vec` of pairs (the Vec is not a
    root once the callback has emptied the source). When the source is the table being
    refilled (`rehash`), the pairs move to a Ruby Array first and are read back by index.
15. **Native payload state as a `Copy` snapshot.** When every field of the payload is `Copy`
    (`REnum`), reading is `fn renum(c: &Ctx, v) -> REnum { *c.data::<REnum>(v).expect(..) }`
    and writing is `renum_mut(c, v).pos += 1`. Code that kept `let en = renum(v).unwrap();`
    across ten calls keeps its shape; what changes is that a field read after a call is the
    value from before it, so each such read was checked against what the call can write.
16. **Allocation inside an argument list**: `c.ary_new_from(&[c.float_new(x), y])` is E0499.
    Name the temporaries (`let f = c.float_new(x); c.ary_new_from(&[f, y])`); the temporary was
    an unrooted Value in a Rust array during the second allocation, now it is a local.
17. **Boot-time `init`** has no context. It takes the classes it needs and returns the ones it
    defines (`vm.c_struct = structs::init(vm.c_object, vm.m_enumerable)`,
    `vm.ids.r_each = range_methods::init(vm.c_range)`); values that must be allocated at boot
    (Float::INFINITY) are made by `builtins::init`, which is not converted. Order matters when
    a subclass inherits state at creation: Enumerator is made, and marked "no allocator", by
    `builtins::init` before `enumerator::init` defines its subclasses.
18. **A context-free function with callers outside the module** (`hash_pairs(h)`,
    `int_from_i64(i)`): the module gets the `Ctx` method; the old body moves to a module that
    is not converted (`object.rs`, `legacy.rs`) and is re-exported from the converted module's
    outer scope, next to `mod imp { .. }` as in array.rs, so that `crate::hash::hash_pairs`
    still resolves for the callers that are left.
19. **Count the accesses on a hot path.** Each accessor call is a kind check that the compiler
    cannot merge across a call. Two shapes cost more than the pilot's "two instructions":
    * a sequence of small accessors on one object (`hash_aset`: flags, identity, lookup, write
      was +36 instructions per `h[i] = v`): take **one** borrow and do the whole operation on
      it (`let r = c.hash_mut(h); r.find_imm(k) .. r.insert_new(..)`);
    * a Rust block whose state is a Ruby Array (`run_each(c, recv, |c, d, args, _| ..)` reads
      the result Array and the block out of `d` at every element: +9 per element): keep the
      state in locals where the iteration can be written as a loop
      (`Enumerable#map` on an Integer Range).
20. **A function that got bigger stops being inlined** (`hash_aset` into the hash-literal
    node: +3 % on nesteddata until it was split into an `#[inline]` front and an
    `#[inline(never)]` String-key part). Compare instruction counts per commit, not at the end.

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

Met in the third wave: patterns 21 to 28 in "Third wave", above. Met in the fourth wave:
patterns 29 to 35 in "Fourth wave", above.

## What is still open

* The call sites in the table below (498, after the fourth wave; 449 after the method layer,
  `heap_audit.rb`): rt.rs, vm.rs, object.rs, node.def, lib.rs, str_legacy.rs, legacy.rs (which
  now also holds the three boot helpers), gc.rs, aot.rs. Every module under `src/builtins/` is
  converted (section "Method layer").
* The OS interface: `builtins/sys.rs` (67 `unsafe`, each a wrapper with its justification). What
  the wrappers cannot discharge is listed in "Fourth wave": `fork`, the environment functions,
  `signal`.
* `heap::Global` values live in `Vm.pinned` slots, which nothing ever frees: fine for the module
  globals it replaces, not a general handle mechanism.
* The frame surface. Converted modules read the chain through `caller_frame` / `cur_frame` and
  write it only through `set_caller_visi` / `set_caller_cref`, but `Ctx::frame() -> &'static mut Frame`
  and the raw `*const Block` / `*const MethodEntry` / `*const Cref` graph stay as they are in
  vm.rs (another work item: safe_stack.md). In particular H5 and the in-place writes
  (`me_set_visi`, the frame setters) assume the evaluator holds no Rust reference to the same
  frame or entry across the builtin's call; that is not checked.
* `static mut VM` (`vm()`), `static mut REFINEMENTS_ON`, and the other `static mut` of vm.rs,
  gc.rs, fiber.rs, kalloc.rs, object.rs, rt.rs, aot.rs (20 left; the builtins have none).
* A converted module still calls context-free functions of unconverted ones while holding no
  borrow: `ivar_get` / `ivar_set_raw`, `const_set`, `respond_to`, `new_module` / `new_class`
  (allocate without the context), `crate::string::str_to_inum` (may allocate a bignum: callers
  copy the bytes first), `recursion_enter` (now in legacy.rs).
* Allocation behind the context's back (`gc::alloc` outside heap.rs; boot-time `init`s use it on
  purpose, pattern 16 of the String side), until `gc::alloc` is reachable only through `Ctx::alloc`.
* Qualified legacy paths (`crate::hash::hash_new()`) compile in a converted file (pattern 28);
  only `heap_audit.rb` rejects them.
* cfunc arguments `&[Value]`, `rdata` payloads holding Values, `Vec<Value>` scratch buffers:
  rooting, roadmap section 2. This wave found one such bug by GC stress (MatchData#to_a) and
  closed three latent ones by construction (pattern 23); the general hole remains.
* String side: the context-free functions of str_legacy.rs are still called from unconverted
  modules; `no_legacy_str` and `no_legacy_hn` have not been folded into `no_legacy`.
* `sbuf.rs` keeps its `unsafe` (the inline byte buffer): the safe forms measured in "Method
  layer" cost up to +16 % instructions on short-string benchmarks; branch `sbuf-safe-rs29`.

## Remaining work per file

`ruby tools/heap_audit.rb --md` at 52842b4 (after the fourth wave; regenerate after a merge).
Columns: call sites of each old accessor (`rstr` includes `str_bytes`, `rproc` includes
`rmethod`, `ary_*` are the context-free array functions of object.rs); `sum`; functions of the
file that return `&'static mut`; `static mut` declarations; `unsafe` occurrences (comments
excluded). Generated `src/node_*.rs` and `src/prism_ast.rs` are not counted; `node.def`, their
source, is. The context-free String functions of str_legacy.rs are not among the columns; a
file that imports `heap::no_legacy_str` is checked for them.

| file |    obj |   rary |   rstr |  rhash | rclass |  rproc |   rbig |  rdata | ivars_of |  ary_* |     vm |   sum | fn->&'static mut | static mut | unsafe | state |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| src/aot.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      1 |     1 |                0 |          1 |      4 |  |
| src/array.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/bignum.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/builtins/array_methods.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/builtins/binding.rs |      0 |      0 |      1 |      0 |      0 |      1 |      0 |      1 |      0 |      3 |      9 |    15 |                1 |          0 |      9 |  |
| src/builtins/complex.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/builtins/encoding.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/builtins/enum_native.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/builtins/enumerator.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/builtins/env.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/builtins/exception.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/builtins/extras.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/builtins/file.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/builtins/hash_methods.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/builtins/io.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/builtins/json.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/builtins/kernel.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/builtins/marshal.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/builtins/math.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/builtins/misc.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/builtins/mod.rs |      3 |      0 |      0 |      0 |     18 |      0 |      0 |      0 |      0 |      4 |      1 |    26 |                0 |          0 |      2 |  |
| src/builtins/module.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/builtins/numeric_methods.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/builtins/object.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/builtins/pp.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/builtins/prelude.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/builtins/proc.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/builtins/process.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/builtins/random.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/builtins/range_methods.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/builtins/rational.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/builtins/regexp_methods.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/builtins/sprintf.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/builtins/string_methods.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/builtins/stringio.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/builtins/strscan.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/builtins/structs.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/builtins/symbol_methods.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/builtins/sys.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |     67 |  |
| src/builtins/thread.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/builtins/time.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/compile.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      6 |  |
| src/fiber.rs |      0 |      1 |      0 |      0 |      1 |      1 |      0 |      2 |      0 |      2 |     10 |    17 |                2 |          6 |     21 |  |
| src/gc.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      2 |     2 |                1 |          3 |     16 |  |
| src/hash.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/heap.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |     33 |  |
| src/kalloc.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          5 |     12 |  |
| src/legacy.rs |      3 |      0 |      0 |      0 |      0 |      0 |      2 |      0 |      0 |      0 |      5 |    10 |                0 |          0 |      0 |  |
| src/lib.rs |      0 |      1 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      2 |      8 |    11 |                0 |          0 |      2 |  |
| src/numeric.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/object.rs |      9 |      5 |      1 |      5 |      0 |      0 |      0 |      0 |      0 |      0 |      5 |    25 |                7 |          1 |      7 |  |
| src/prism.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      1 |  |
| src/range.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/regexp.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/rt.rs |     22 |     50 |      9 |      4 |     30 |      1 |      0 |      0 |      0 |     14 |     58 |   188 |                0 |          2 |     68 |  |
| src/sbuf.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |     10 |  |
| src/stack_raw.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      5 |  |
| src/str_legacy.rs |      1 |      0 |      7 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      3 |    11 |                0 |          0 |      0 |  |
| src/string.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |      0 | converted, forbid(unsafe_code) |
| src/value.rs |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |      0 |     0 |                0 |          0 |     11 |  |
| src/vm.rs |     24 |      3 |      2 |      5 |     61 |     10 |      0 |      1 |      1 |      7 |     53 |   167 |                2 |          2 |     75 |  |
| node.def |     12 |      0 |      2 |      0 |      0 |      1 |      0 |      0 |      0 |      3 |      7 |    25 |                0 |          0 |     27 |  |
| total |     74 |     60 |     22 |     14 |    110 |     14 |      2 |      4 |      1 |     35 |    162 |   498 |               13 |         20 |    376 |  |
