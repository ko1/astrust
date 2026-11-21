# saruby (ASTrust) roadmap: safe Rust, modular precise GC, koruby_precise speedups

Written 2026-11-20. Three directions set by the project owner, and how they fit together.
Sources: the ideas ledger `~/ruby/src/ideas/` (saruby-*), `~/ruby/astro/sample/koruby_precise`
(C sibling, master d7b4ff45e), `~/ruby/astro/runtime/precise_gc/`, `~/ruby/astro/docs/gc_design.md`.

## 1. Safe Rust as far as it goes

Goal: `unsafe` only at external interfaces (prism FFI, libc, dlopen of baked code, fiber context
switch) and inside a small audited core (GC heap, value stack). Everything else `#![forbid(unsafe_code)]`.
State today: about 1900 `unsafe` occurrences, and several safe-looking but unsound APIs
(`rary(v) -> &'static mut Vec<Value>`, `obj(v) -> &'static mut RObj`, `static mut` method entries).

Plan, in order:
1. **Heap access borrows the context.** `c.ary(v) -> &[Value]`, `c.ary_mut(v) -> &mut Vec<Value>`, etc.
   The borrow of `c` makes "hold a body reference across a call that may allocate" a compile error:
   the aliasing rule and the precise-GC rule become one borrow-checker rule, at no run-time cost.
   The deref inside the accessor stays `unsafe` in the core and is sound as long as a `Value` handle
   is live (see rooting below).
   Core done (2026-11-21, `src/heap.rs`) and piloted on Array: `array.rs` and
   `builtins/array_methods.rs` are `forbid(unsafe_code)`. Cost: the run-time kind check, 0 to
   +2.1 % instructions. Design, numbers, the conversion recipe and the per-file count of what
   is left (2302 call sites): `docs/safe_heap.md`.
2. **Value stack as slices.** Done (2026-11-20) as two backends behind one API: `src/stack.rs`
   (the default since 2026-11-21; `FEATURES=raw_stack` is the raw one; `#![forbid(unsafe_code)]`, frames are `&[Cell<Value>; FRAME]`) and
   `src/stack_raw.rs` (default, raw pointers). The checked one costs too much on call-heavy code to be
   the default; numbers, design and rejected alternatives: `docs/safe_stack.md`.
3. **No `static mut`**: builtin method identities (`BO_EQ_ME`, ...) and the VM singleton go through the context.
   Identities and `MAIN_CTX` are `Vm` fields now (`vm.ids`, `vm.main_ctx`); `c.vm()` exists; `static mut VM`
   and 47 others remain (`docs/safe_heap.md`).
4. **Builtins become `forbid(unsafe_code)`** module by module once 1–3 exist (they are the bulk of the count).
5. Remaining unsafe, each behind a safe API: GC allocator/sweeper, stack scanning (until precise),
   node operand access in generated code, code store loading, fibers, FFI.

What safe Rust cannot give with a recursive tree-walking evaluator: a static proof that every live
`Value` is rooted (gc-arena-style branding only collects when no value is on the Rust stack). So
rooting stays a discipline, as in ASTro, but Rust types can carry most of it (item 1, and slot
references `Ref<'frame>` for values that cross a GC point).

## 2. Modular precise GC (as ASTro's `runtime/precise_gc`)

ASTro: the framework owns header, allocation API, edge visiting, write barrier, weak/finalize;
the language supplies root enumeration and per-object edge enumeration; 15 backends chosen at build
time (`make GC=<name>`). Precision has no stack maps: values live in value-stack slots, C code never
holds a raw value across a may-GC call (checked by CodeQL + GC stress), baked code has the same shape.
Classes are pinned so inline caches keep raw class addresses.

ASTrust plan:
1. `astrust/runtime/gc/`: `trait GcLang` (visit_roots, scan_edges, finalize, is_gc_object) and
   `trait GcBackend` (alloc, collect, store barrier, pin, stats); edges are visited as `&mut Value`
   so moving backends can update them. Backend chosen at compile time (cargo feature / type alias).
2. Port today's non-moving mark-sweep as the first backend. "Ambiguous roots" (conservative machine
   stack) is a mode of the binding, so nothing regresses while precision is introduced.
3. Generator hook for where `@child` values live (default: Rust local; precise embedders: a value
   stack slot), the counterpart of astrogen's `child_storage_*` hooks.
4. GC stress mode that collects at every allocation, as the test for the rooting discipline
   (`SARUBY_GC_STRESS` exists; 9 known failing tests under it).
5. Then precise roots (drop machine-stack scanning), then a moving / generational backend
   (needs: address-independent object_id/hash, payloads owned by the GC heap instead of `Vec` + `Drop`,
   pinned classes or cache flush, nil-filled or zeroable slots).
Blocking assumptions in today's code are listed in section 4.

## 3. Speedups studied in koruby_precise, by expected value here

Already ported: per-site class+serial cache, callee entry node, speculated callee inlining, 4-way
polymorphic table, Klass.new fast path, attr / cfunc cache kinds, condition-mode compares, dense
literal kwargs, native Enumerable, fast lead-only block yield, lite frames, size-class allocator,
small strings in the slot, lazy backtraces, ivars in the slot (item 1 below, without shapes: the
class is the shape and slots are prefilled), `when Klass` memo (item 7, 8 entries per site).

Adopted in koruby_precise and not here yet (ledger slug; mechanism; their number):
1. **Ivars embedded in the object + shape/transition cache** (`saruby-obj-ivar-embed-self`,
   `-ivar-prealloc-new`, `-ivar-hint-max`, `-ivset-hit-or-transition`, `-ivar-set-imm-no-wb`): one
   allocation per object, capacity max(4, class hint); set cache `{shape, slot, from_shape}` with one
   combined test. new_obj instructions -20%; ivar_set branch misses -68%. → object, ivar, gcchurn, binary_trees.
2. **Runtime closure escape** (design): a block reads outer locals from the definer's live frame; an
   Env is made only when a Proc is actually created, copied to the heap only if it escaped at frame
   exit. Our compile-time heap Env allocates on every call of a method with captured locals.
   → closures, iterators, gcchurn. Measured bound (2026-11-21, after small Envs moved into the slot):
   a method with a block that captures a local and goes to `Array#each` costs 148 instructions /
   ~36 cycles per call more than one without (`b_env_cap` / `b_env_nocap`); that is all a lazy Env
   could save there. Not done (needs blocks to read the defining frame).
   Small Envs in the slot done (2026-11-21, `src/envvals.rs`): b_env_cap 230 → 148 extra
   instructions per call, closures -12 % instructions.
3. **Block entry + inline yield** (`saruby-blk-entry-yield-inline`, `-invoke-blk-simple`,
   `-yield-frame-link`, `-uses-block-entry`): 383 → 195 insns; block -6.49%, iterators -6.52%.
   Block entry done (2026-11-21, `node_block_entry`): block -7.1 %, iterators -6.9 %, mapreduce
   -5.2 % instructions. Not done: compiling the block body into a `yield` of a callee that is
   itself compiled into the site (needs the generator to pass the block's type into the callee).
4. **Keyword entries** (`saruby-kw-dense-entry`, `-kw-dense-call`, `-kw-defaults-entry`): 254 → 105
   insns; defaults 370 → 63. → kwargs. Done (2026-11-21, kentry / dentry): a dense keyword call
   435 → 67 instructions, kwargs 3978.8M → 1164.6M instructions.
5. **Call path leftovers** (`saruby-fib-call-path`, `-call-return-cold-tail`, `-entry-inline-return`):
   nothing live across the call, line recovered from the frame in a cold tail, stack check only when a
   high-water mark moves. fib 17.08 → 12.04 cycles/call. Also: cache validity without a serial
   compare (caches registered per method name, flushed on redefinition).
   Done here (2026-11-21, `docs/design.md` "Calls, frames and blocks"): frames without an id, no
   frame for leaf bodies, one sp store less per call, the native check out of compiled-in
   entries, the serial as a static. A self call of `def f; @a; end` 96 → 72 instructions,
   `def f(x); x + 1; end` 73 → 65; optcarrot 131.5M → 128.6M instructions per frame. Left: the
   line store per call (2 instructions; every design found still needs a store), the stage
   check of the checked stack (8), the cache test (11), the frame record (11).
6. **Embedding for Array (<= 6) / Hash (<= 4 pairs)** (`saruby-object-embedding`), branch-free reads.
7. **case/when class memo** (`saruby-kind-of-site-memo`): hit is `(memo ^ class) <= 1`. casecls 257 → 197.
8. **Builtin sends inlined at the site** (`saruby-aot-inline-builtin-sends`, about 61 names):
   optcarrot AOT -5%, sudoku -44% instructions.
   Done for 21 bodies (2026-11-21, `src/inline_builtin.rs`, `docs/design.md` "Calls"): collatz -62 %,
   array_access -42 %, render_span_kernel -21 %, methodchain -19 % instructions; optcarrot -2.3 % per
   frame (mostly Integer#[]). Not done: `send` (optcarrot's largest builtin call left, 20K per frame),
   `a[i, n] = x`, `rotate!`, builtins taking a block (`times`, `each`), polymorphic sites.
9. **String append / interpolation in place** (`saruby-str-shl-inplace`, `-interp-piece-inplace`),
   strscan with String patterns as memcmp (`saruby-strscan-*`).
10. **Same-method polymorphism** (`saruby-ic-same-method-poly`): keep the key when subclasses reach
    the same method. lambda.call inline (574 → 160), `&blk` forwarding without a Proc, baked `ensure`.
    lambda.call done in part (2026-11-21): through the lambda's block entry, 306 → 219 instructions
    (the `Proc#call` cfunc and `call_proc_kw` remain in the path).
11. Bump allocation inline + adaptive nursery (`saruby-copy-gen-young-adaptive`): needs a moving GC (section 2).

Rejected there (do not repeat without a new reason): caller-side invoker table (+15〜24% wall);
callee body expansion at the site in the general case (instructions -4.1% but cycles +4% on optcarrot;
we do a bounded form, re-measure with cycles not instructions); per-site PIC on railsbench (-0.19%);
cache version tags; direct allocation expanded at the `new` hit (gcbench +3.8%); lvar op-assign fusion
(cycles +8.7%); `-Os` for baked code; 64-byte aligned caches; nil-fill skipping; operators with
per-site caches (fib +4.5%); float division out of line (floatcalc +21.1%).
Lesson recorded many times in the ledger: instruction counts and cycles disagree in sign; decide on time.

## 4. What today's code assumes (and a precise or moving collector breaks)

- `Value`s in Rust locals are safe (conservative machine-stack scan): cfunc signature
  `fn(&mut Ctx, Value, &[Value], *const Block)`, generated `@child` locals.
- `&'static mut` borrows into object bodies (`object.rs` accessors; Array methods no longer, see `docs/safe_heap.md`).
- Values copied into Rust heap buffers across calls (`args.to_vec()`), invisible to any scan.
- Addresses as identity: `Object#hash`, `object_id`, inspect, `gen_ivars`, method cache keys,
  `CallCache.klass` / `IvarCache.klass` raw in nodes.
- Sweep runs `Drop` on dead bodies (payloads are `Vec` / `Box` / `HashMap`).
- `Value` is `NonZeroU64` with nil = 0x04: zero-filled memory is not valid.

## Open measurements
- tak / ackermann AOT instruction counts (local): 5413M / 3874M at a373e61, 5711M / 4377M at a358117,
  5751M / 4454M after the merges. A bisect over the string/GC performance branch on its own base
  shows it adds only ackermann +3〜5%, tak +0.6%; the rise happened earlier, with the shared
  `cc_fast` / polymorphic table / cfunc cache kind in the call path (fib unchanged). sp4 wall time
  over the same span still improved (tak 0.337 → 0.298 s), so this is a possible small win to
  recover, not a regression in time. (An earlier note here blamed the merge; that was wrong.)
- **`gc::alloc` copies the body through the stack.** The caller's `Body` stays on the stack across
  the may-GC `slow_path()` call and is copied into the slot with a 32-byte load right after its
  1/8-byte stores: a store-forwarding stall per allocation of any kind. Returning into an
  out-of-line `alloc_slow(klass, body)` on the slow condition, so that the fast path has no call
  and stores the fields from registers, measured (cycles:u, AOT, instructions unchanged):
  gcchurn 700M → 521M, nesteddata 292M → 231M, gen_gc 718M → 636M, object 676M → 563M,
  binary_trees 532M → 483M. Not applied here (2026-11-20): gc.rs was being moved to the framework.
- **Arrays of up to 3 elements in the slot**: done (2026-11-21, `src/avec.rs`) on top of
  `alloc_with`, with Array literals stored from registers and sweep skipping `Drop` for bodies
  that own nothing. Measurements, the costs on index-heavy code (fannkuch / sieve AOT +5〜7 %
  instructions, cycles within noise) and what did not help (lazy sweep: +1 % instructions, no
  cycle gain at these heap sizes, the heap sits in L3): `~/ruby/src/trials/2026-11-21-saruby-alloc-sweep/`.
  The marksweep heap is kept at 3x the live set (was 2x): gen_gc -11 % cycles for +15 % max RSS.
- Tried for ivars and dropped: a combined "hit or append" store test on the Vec representation
  (object AOT instructions 1978M → 2002M, ivar 4539M → 4827M); prefilled slots without embedding
  (object 1978M → 1986M: the gain is the malloc, not the test); 4 packed slots with a
  data-pointer select per read (ivar 4539M → 4763M, nbody 3071M → 3189M).
- Interpreter (`--plain`) cycles move by several percent with code and node placement at equal
  instruction counts (poly 1373M → 1494M when the `when` memo was added, which poly never runs;
  +1% / +5% / +3% with 4 / 8 / 16 memo entries). Compare instruction counts first there.
- tak / ackermann AOT instructions rose with the string/GC performance merge (tak 5413M → 5751M,
  ackermann 3874M → 4454M, fib unchanged); not the mixed Float/Integer inline path (4454M → 4372M
  with it removed). Cause not found yet.
