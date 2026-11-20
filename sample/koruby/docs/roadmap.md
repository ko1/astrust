# koruby (ASTrust) roadmap: safe Rust, modular precise GC, koruby_precise speedups

Written 2026-11-20. Three directions set by the project owner, and how they fit together.
Sources: the ideas ledger `~/ruby/src/ideas/` (koruby-*), `~/ruby/astro/sample/koruby_precise`
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
2. **Value stack as slices.** Done (2026-11-20) as two backends behind one API: `src/stack.rs`
   (`FEATURES=safe_stack`, `#![forbid(unsafe_code)]`, frames are `&[Cell<Value>; FRAME]`) and
   `src/stack_raw.rs` (default, raw pointers). The checked one costs too much on call-heavy code to be
   the default; numbers, design and rejected alternatives: `docs/safe_stack.md`.
3. **No `static mut`**: builtin method identities (`BO_EQ_ME`, ...) and the VM singleton go through the context.
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
   (`KORUBY_GC_STRESS` exists; 9 known failing tests under it).
5. Then precise roots (drop machine-stack scanning), then a moving / generational backend
   (needs: address-independent object_id/hash, payloads owned by the GC heap instead of `Vec` + `Drop`,
   pinned classes or cache flush, nil-filled or zeroable slots).
Blocking assumptions in today's code are listed in section 4.

## 3. Speedups studied in koruby_precise, by expected value here

Already ported: per-site class+serial cache, callee entry node, speculated callee inlining, 4-way
polymorphic table, Klass.new fast path, attr / cfunc cache kinds, condition-mode compares, dense
literal kwargs, native Enumerable, fast lead-only block yield, lite frames, size-class allocator,
small strings in the slot, lazy backtraces.

Adopted in koruby_precise and not here yet (ledger slug; mechanism; their number):
1. **Ivars embedded in the object + shape/transition cache** (`koruby-obj-ivar-embed-self`,
   `-ivar-prealloc-new`, `-ivar-hint-max`, `-ivset-hit-or-transition`, `-ivar-set-imm-no-wb`): one
   allocation per object, capacity max(4, class hint); set cache `{shape, slot, from_shape}` with one
   combined test. new_obj instructions -20%; ivar_set branch misses -68%. → object, ivar, gcchurn, binary_trees.
2. **Runtime closure escape** (design): a block reads outer locals from the definer's live frame; an
   Env is made only when a Proc is actually created, copied to the heap only if it escaped at frame
   exit. Our compile-time heap Env allocates on every call of a method with captured locals.
   → closures, iterators, gcchurn.
3. **Block entry + inline yield** (`koruby-blk-entry-yield-inline`, `-invoke-blk-simple`,
   `-yield-frame-link`, `-uses-block-entry`): 383 → 195 insns; block -6.49%, iterators -6.52%.
4. **Keyword entries** (`koruby-kw-dense-entry`, `-kw-dense-call`, `-kw-defaults-entry`): 254 → 105
   insns; defaults 370 → 63. → kwargs.
5. **Call path leftovers** (`koruby-fib-call-path`, `-call-return-cold-tail`, `-entry-inline-return`):
   nothing live across the call, line recovered from the frame in a cold tail, stack check only when a
   high-water mark moves. fib 17.08 → 12.04 cycles/call. Also: cache validity without a serial
   compare (caches registered per method name, flushed on redefinition).
6. **Embedding for Array (<= 6) / Hash (<= 4 pairs)** (`koruby-object-embedding`), branch-free reads.
7. **case/when class memo** (`koruby-kind-of-site-memo`): hit is `(memo ^ class) <= 1`. casecls 257 → 197.
8. **Builtin sends inlined at the site** (`koruby-aot-inline-builtin-sends`, about 61 names):
   optcarrot AOT -5%, sudoku -44% instructions.
9. **String append / interpolation in place** (`koruby-str-shl-inplace`, `-interp-piece-inplace`),
   strscan with String patterns as memcmp (`koruby-strscan-*`).
10. **Same-method polymorphism** (`koruby-ic-same-method-poly`): keep the key when subclasses reach
    the same method. lambda.call inline (574 → 160), `&blk` forwarding without a Proc, baked `ensure`.
11. Bump allocation inline + adaptive nursery (`koruby-copy-gen-young-adaptive`): needs a moving GC (section 2).

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
- `&'static mut` borrows into object bodies (`object.rs` accessors, `array_methods.rs` `modify`).
- Values copied into Rust heap buffers across calls (`args.to_vec()`), invisible to any scan.
- Addresses as identity: `Object#hash`, `object_id`, inspect, `gen_ivars`, method cache keys,
  `CallCache.klass` / `IvarCache.klass` raw in nodes.
- Sweep runs `Drop` on dead bodies (payloads are `Vec` / `Box` / `HashMap`).
- `Value` is `NonZeroU64` with nil = 0x04: zero-filled memory is not valid.

## Open measurements
- tak / ackermann AOT instructions rose with the string/GC performance merge (tak 5413M → 5751M,
  ackermann 3874M → 4454M, fib unchanged); not the mixed Float/Integer inline path (4454M → 4372M
  with it removed). Cause not found yet.
