# Moving collectors for ASTrust / saruby: precise roots, `copy`, `copy_gen`

2026-11-25.  Design (phase 0) and the first slice of phase 1.  Trial record:
`~/ruby/src/trials/2026-11-25-astrust-moving-gc/`.

Goal: `make GC=copy` (Cheney semi-space) and `make GC=copy_gen` (two generations, N-survive,
remembered set, write barrier) in `sample/saruby`, as **fully moving** collectors with **precise
roots**: no conservative scan of machine stacks, no pinning driven by stack words.  The reference
is ASTro (`~/ruby/astro/docs/gc_runtime.md` §2.4 / §2.5, `runtime/precise_gc/`) and the rooting
discipline of `~/ruby/astro/sample/koruby_precise/docs/rooting_guide.md`, which made a C Ruby run
under a moving collector.  The framework side (`runtime/gc`, `docs/gc.md`) already has what a
moving backend needs (`MOVING`, edges as `&mut Ref` slots, `from_object`, `pin`, `barrier` /
`store`); what blocks it is saruby's code, listed in `docs/gc.md` "What still blocks precise roots
or a moving backend".  This document says how each item is resolved, in what order, and at what cost.

Contents: 1 the end state · 2 the rooting discipline · 3 making each kind of root precise ·
4 the heap: moving space and non-moving space · 5 address-as-identity · 6 backend `copy` ·
7 backend `copy_gen` and the write barrier · 8 the blocker list, item by item · 9 migration order,
cost, validation · 10 fallback: mostly-copying · 11 phase 1 slice (what is implemented now).

## 1. The end state

* Roots are exact: the VM value stacks up to each context's `sp`, the frame chain (`Frame` records
  and the `Block`s they point at), parks (section 2.3), the `Vm` tables, and the payloads of heap
  objects (`scan_edges`).  `GcLang::AMBIGUOUS_ROOTS = false`; `stack::with_current_stack` is no
  longer called; a suspended fiber's machine stack holds no Value the collector needs.
* Every heap object in the moving space can change address at any GC point.  A GC point is any
  call that can allocate from the GC heap or run Ruby code (section 2.1).
* Objects of a few kinds live in a non-moving space chosen at allocation (section 4): classes and
  modules, and the values AST nodes and baked AOT code hold by address.  That space is collected by
  mark-sweep inside the copying collection.  It is a choice by kind, not pinning by stack words.
* Payloads stay Rust-owned (`Vec`, `Box`): only the fixed-size `RObj` slot moves (a bitwise copy;
  the payload pointer travels with it).  Dead objects still need `finalize` (section 6.3).
* The default build (`make`, marksweep, conservative) is unchanged until a measurement shows the
  precise discipline costs ≤ 0.3 % per benchmark; until then the precise parts that cost
  instructions are compiled only into precise builds (cargo feature `precise`, implied by
  `GC=copy` / `GC=copy_gen`).  The parts that cost nothing (types, roots visited at GC time) are
  in every build.

## 2. The rooting discipline (Rust form of koruby_precise's)

### 2.1 What moves, what is a GC point

* A *moving handle* is a heap `Value` in the moving space.  Immediates (Fixnum, flonum, static
  Symbol, nil/true/false, UNDEF) never move; non-moving-space objects (section 4) never move.
  Code cannot tell a non-moving object from a moving one by looking, so the rule treats every heap
  Value as moving, except where the type says otherwise (a `Value` known to be a class).
* A *GC point*: `gc::alloc*` and everything that can reach it (`str_new*`, `ary_new*`, `hash_new`,
  `float_new`, `big_to_value`, `intern` of a dynamic symbol, `singleton_class`, ...), `funcall`,
  `yield` / block calls, `raise` (builds a backtrace), `inspect` / `to_s` / `eql?` / `hash` of
  arbitrary objects, `const_get` with autoload, `require`.  Not GC points: reading bodies, Rust
  heap growth (`Vec::push`), `malloc_increase` (it only accounts; the collection waits for the next
  allocation).
* In Rust every GC point takes `&mut Ctx` (safe_heap.md made that true for allocation; every
  `funcall` / `yield` path takes it already).  The converse is not true (`ary_push` takes `&mut Ctx`
  and never collects), which the compile-time help below accepts as over-approximation.

### 2.2 Where roots live

As in koruby_precise, heap values that must survive a GC point live in **slots the collector
visits**: value-stack slots below `sp`, frame records, parks.  A Rust local holding a moving handle
is valid until the next GC point and stale after it.  Rules (numbered as koruby's idioms):

* **A. Re-read from the slot.**  `self` and the arguments of a call are staged in value-stack
  slots below `sp` (`funcall_flags_raw` / `stage`: "the staged originals stay below sp").  After a
  GC point, read them again from the slot; return the re-read value, not the entry-time local.
* **B. Helpers take slots, not values, when they allocate.**  koruby's "helper は VALUE * で受ける":
  a helper that allocates and then uses an input takes `&Slot` (`Cell<Value>`, `src/slots.rs`) and
  re-reads it after its allocation.  A helper that allocates first and uses its inputs only to build
  the new body may take values: the body passed to `alloc` is a root during the call (`pending`).
* **C. Park.**  Values without a slot (an intermediate result, a cursor, a value read from a payload)
  are parked across a GC point in value-stack slots taken by a **scope**: `c.park(n, |c, p| ...)`
  reserves n slots above `sp`, initialized to nil, runs the closure, and restores `sp` on every exit
  (the closure's return, `?`, unwinding).  This is koruby's frame park (IDIOM C) with value-stack
  slots instead of two frame fields: there is no fixed limit of two.
* **E. Re-fetch from a stable container** each time (a constant table, the receiver's ivar).

Why the park is not koruby's forbidden IDIOM B ("bump `c->sp_top`"): koruby forbade writing the root
top because two sources of truth (the threaded `sp` and `c->sp_top`) went out of sync — a caller bumped
it, an inner helper reset it, the caller's slot fell out of the scan.  In saruby `Ctx.sp` is the single
source of truth (there is no separate scan top: `Ctx::live()` is `[0, sp)`), and the rule that makes
it safe is structural: `sp` is moved only by LIFO scopes — call staging (`stage` / `at_entry` /
`enter`), whose callers restore it, and `park`.  `sp_reset(mark)` with a mark taken by someone else is
the koruby bug; phase 1 audits its 85 call sites (`grep -n 'sp_reset\|set_sp'`) and phase 2 makes the
raw setters private to `stack.rs` / `vm.rs`, so builtins can only park.  koruby's P1 bug ("yield
restores sp_top to the call level and drops the park") needs a yield that runs the block below the
parking scope.  Whether any saruby yield path does (a block whose env frame is reused below the
current `sp`) is not established; it is a hypothesis to test, and the distinguishing observation is
a park read after a yield under STRESS + CHECK (phase 1a does this in `filter_in_place` / `uniq_ary`,
which park across `yield1`).

### 2.3 Compile-time help

Rust can make part of the discipline a borrow error, in the way safe_heap.md made "a body
reference held across a GC point" one:

* **Body references** (`c.ary(v) -> &[Value]`, `c.hash_mut(v)`) already borrow `c`; they cannot
  live across a GC point.  With a moving collector this is also what keeps them from pointing into a
  from-space slot.  Done (safe_heap.md waves 1-4).
* **Parked slots are branded.**  `c.park(n, |c, p: Park<'s>| ...)` is `for<'s>`: a `Park<'s>` and the
  `PSlot<'s>` handles it gives cannot leave the closure, so a slot cannot be used after the scope
  returned its slots (which would read a stale Value above `sp`).  Slots are `Cell`s: reading one
  needs no borrow of `c`, so `p.get(i)` after a GC point is fine — that is the point of a slot.
* **Local values carry a borrow.**  `c.local(v) -> Local<'_>` (a `Value` with a `PhantomData<&Ctx>`):
  a `Local` is dead at the next `&mut Ctx` call (any GC point), so "used after a GC point" is E0502.
  Method syntax keeps passing it ergonomic: `c.ary_push(a.v(), x.v())` evaluates the arguments under
  the two-phase reservation of `c`; using `a` again on the next line is the error.  A converted file
  binds heap values as `Local` (or parks them); `heap_audit.rb` gains a column counting `let` bindings
  of type `Value` in converted files that are not immediately consumed — a lint, not a proof.
* **What cannot be expressed.**  `Value` itself is `Copy` and has no lifetime; a raw `Value` in a
  local across `&mut Ctx` compiles.  Making `Value` lifetime-branded everywhere (`Value<'gc>`, the
  gc-arena model) would make the whole discipline a type error, at the cost of touching every one of
  ~90k lines; it is the long-term option, not the plan.  The enforcement is the runtime gate, as in
  ASTro: **STRESS + PURGE** (2.5) turns every stale use into a deterministic crash, and the compile-time
  pieces reduce how many such crashes there are to find.
* **Handles** (a table of indirections) are used only at the external boundary: values held by
  foreign code (none today; C extensions if ever) — as koruby's c_ext_api_design.md.

### 2.4 Values in Rust heap buffers

`args.to_vec()`, `collect::<Vec<Value>>()`, a `Vec<Value>` built in a loop and used after a call:
invisible to any scan (koruby's "libc shadow buffers", its `ary_concat` bug).  Each is rewritten to one
of: index iteration re-reading the source slot each step (CRuby's own iteration, and usually the
CRuby-compatible behaviour for a source mutated by the block), a Ruby Array parked in a slot (one
allocation), or a park of n slots (`c.park(n, ...)` copying the values in).  The inventory is in
section 8.

### 2.5 Validation: STRESS + PURGE, and before that CHECK

* **STRESS**: `SARUBY_GC_STRESS=1` / `GC.stress = true` — a collection at every allocation (exists).
* **PURGE** (with `GC=copy`): every collection `mprotect`s the from-space pages `PROT_NONE` and
  keeps them out of reuse for a quarantine budget (round robin, as ASTro's `BARUBY_GC_PURGE`); any
  stale handle use faults at once, whether or not the object is alive elsewhere.
* **CHECK** (phase 1, before `copy` exists; `make FEATURES=gc_check`, marksweep): freed slots are
  quarantined (not reused for a number of collections) and poisoned (`klass` points at a
  `PROT_NONE` page, the body is `Free`), and the conservative machine-stack scan is limited to the
  frames outside precise zones (section 11).  It catches a missed root whose object has no other
  reference (it is freed and the next use faults); it cannot catch a stale handle to an object that
  is still alive elsewhere — only a moving collector can.  It is the gate for converting code before
  the copy backend exists, and a harness check: a deliberately unrooted value must crash under it.
* **Audit build** (later): koruby's RESULT-audit — a logical GC clock ticked at every GC point, values
  stamped when read from a slot, a debug assertion when a value read before the last tick is stored
  into a heap object or a root.  Names the file:line instead of the far-away fault.

## 3. Making each kind of root precise

| root today (ambiguous) | precise form | cost on hot paths |
|---|---|---|
| machine stack: cfunc locals (`fn(&mut Ctx, Value, &[Value], Blk)`) | cfunc ABI `fn(&mut Ctx, Args<'_>, Blk) -> RV` with `Args { base: Stk, argc }`: recv and args are re-read from the staged slots (idiom A); the `&[Value]` copy made by `with_args` goes away (it saves work). Interim: converted cfuncs on the old ABI park their recv/args at entry (section 11). | ABI change: none or negative (no copy). Converted bodies: a re-read per GC point (one load). |
| machine stack: generated node code (`@child` values in Rust locals) | `child_storage_stmt` / `child_storage_expr` overridden in `saruby_gen.rb`: `@child` values go to frame temp slots `fp[base + i]` (ASTroGen's hook exists for this). Only operands evaluated before another child that can GC need it; immediates-only nodes (most Integer arithmetic after specialization) can keep registers when the generator can prove the earlier child is immediate. | a store + load per child; measured before enabling in the default build; under feature `precise` otherwise. |
| machine stack: `Block` / `Frame` structs | walk the frame chain from `c.frame` (and each suspended fiber's `Ctx`): per frame `visit_block(frame.blk)`, `visit_me(frame.me)`; `Block.outer_blk` points at a block reachable the same way. `Frame.fp` points into the value stack (already a root). | GC time only. |
| VM value stacks (ambiguous: slots below `sp` no frame wrote yet hold stale words) | exact: every slot in `[0, sp)` holds a valid Value. `enter` / `enter_base` already nil-fill parameters' tails in most paths (`nil_fill`); the remaining gaps are frames whose temp slots are written lazily. Either nil-fill them at entry or record each frame's written extent. | nil-fill of temps: a few stores per call; measured (the frame sizes are small). |
| Rust heap buffers | section 2.4 | none (rewrites at the sites). |
| statics / thread_locals holding Values (`TOP_WRAPPER`, `CATCH_TAGS`, `LOADING`, `PRIVATE_REF`, `ORIG_ERR`, `ZSUPER_OWNER`, `BI_ME`, ...) | visited from `vm::visit_roots` (or moved into `Vm` fields that are) | GC time only. |
| `RData` payloads | every `RData::visit` covers every Value field (inventory, section 8) | GC time only. |
| `MethodEntry` / `Cref` (Box, outside the heap) | visited precisely already (`visit_me`, `crefs`); the `*const` → `*mut` cast becomes a `Cell<Value>` field so the write through a shared pointer is sound | none. |

## 4. The heap: moving space and non-moving space

Allocation sites choose the space by kind (`gc::alloc_fixed(c, klass, body)` vs `gc::alloc`), decided
statically at the call site, so the fast path of ordinary allocation is unchanged.

**Non-moving space** (mark-sweep inside the copying collection, like many production VMs' "immortal" or
"large object" spaces):

* **Classes, modules, singleton classes, iclasses** (`Body::Class`).  Their addresses are in
  `CallCache.klass` / `spec_klass`, `IvarCache.klass`, `KindMemo`, method-cache keys, `cd.ancestors`
  (an `Rc<Vec<Value>>` shared between classes), and baked AOT code compares receiver classes with
  constants.  Updating all of those after every move would be a hot-path invalidation (or a walk of
  every node and every AOT patch); classes are a small, long-lived population (below), which is
  what a non-moving space is for.  CRuby moves classes and updates caches; we choose not to.
* **Values held by AST nodes**: frozen literals (`srb_fstr`, `frozen_lits`), regexp literals, Range
  literals, Bignum literals (`LitCache.v`), `node_values`, `ConstCache.val` targets are arbitrary
  objects — for those the cache is reset on move instead (section 5), because a constant can hold any
  object.  Literals are allocated by the node's first execution (`srb_*_lit`), so the site knows.
* **Dynamic Symbols' backing strings** (`sym_names`): read from nodes by `Symbol#name` caching.
* **Encoding objects** (`Vm.enc_objs`, indexed by encoding id; rooted through `pinned`): a fixed, small
  set read by index on hot paths.  The tables that copy class or Encoding addresses (`enc_objs`,
  `inline_builtin` `BI_ME`, thousands of entries) are not visited as roots: they hold classes and Encodings
  rooted elsewhere, and with both non-moving the copies stay valid.  (An earlier note here blamed BI_ME's
  visit for a str +2.3 % instruction rise; removing it changed nothing, so that was wrong; see the trial README.)
* Possibly later: large payload owners if a size class is added; Data objects whose payload is
  shared with the OS (none moves today: payloads are Boxes, only the slot moves).

**Cost.**  Fragmentation: non-moving pages are swept, free slots are reused by later fixed
allocations only (a free list per space).  Classes and modules: 118 after boot
(`ObjectSpace.each_object(Module).count` under `-e`, 2026-11-25), growing with the program (singleton
classes); each `RObj` is 56 bytes.  Literals are bounded by source size.  Collection: the
copying collector marks non-moving objects when it reaches them (a mark bit, no copy) and scans their
edges (the edges are rewritten, the object stays); after evacuation the non-moving pages are swept
(finalize the unmarked).

## 5. Address as identity

| use | resolution |
|---|---|
| `gen_ivars` keyed by address (`vm.rs ivar_set_raw`, `gc.rs visit_gen_ivars` / `forget`) | rekey on move: `GcLang::moved(from, to)` (new hook, called by a moving backend after copying an object whose flags have `FL_EXIVAR`); the flag test is on the copied header word, the common object pays one test per copy. Alternative: move the ivars into the object (an `IvBuf` slot in the body for kinds that have room); not needed for correctness. |
| `object_ids` keyed by address (`vm::object_id`) | rekey on move (`FL_HAS_ID`, same hook). The id is a counter, so it does not change. `ObjectSpace._id2ref` uses the reverse map, keyed by id: unchanged. |
| objspace allocation tracer keyed by address (`FL_TRACED`) | rekey on move (same hook). |
| `Object#hash` / `Kernel#hash`, identity tables (`hash.rs identity_hash`, `hash_of_obj` kernel case, `kernel.rs:911`) | the hash must not change when the object moves: derive it from the object id (`object_id(v)`, which then rekeys) — CRuby's approach since compaction. Cost: a table lookup per identity hash of an object that has no id yet; cached by the `FL_HAS_ID` flag test. Identity hash tables then need no rehash. |
| `recursive_guard` (`(u64 addr, Sym)` across `inspect`) | store Values and visit them as roots (a `Vec<(Value, Sym)>` in `Vm`, visited by `visit_roots`). |
| marshal `Dumper.objs: HashMap<u64, usize>`, ractor `seen` / `rec` | key by object id (`object_id`) or keep the objects in a Ruby identity Hash parked in a slot (the Hash's identity hash is then id-based). |
| `CallCache.klass`, `IvarCache.klass`, `KindMemo`, `spec_klass`, method cache keys, AOT receiver checks | classes are in the non-moving space (section 4). |
| `ConstCache.val`, `LitCache.v` | `LitCache` targets are allocated in the non-moving space (section 4). `ConstCache.val`: the cache stores the Value in a visited slot (the caches are reachable from the node tree: a list of live `ConstCache`s visited by `visit_roots`, like `node_values`), so the collector updates it. |
| `node_values` / `frozen_lits` / `symproc_cache` copies inside nodes | the referents are non-moving (section 4); the tables stay roots. |
| `data_free_hooks: Vec<fn(*mut RObj)>` | no user (it is never pushed); remove. |
| `inspect`'s `0x...` addresses | change after a move, as in CRuby with compaction; nothing depends on them. |

## 6. Backend `copy`

`runtime/gc/copy.rs`.  Cheney semi-space over pages, as ASTro's `gc_copy.c` but on 64 KiB pages
(`pages.rs`) rather than two 64 GiB reservations, so that from-space pages can be returned to the pool
one by one and PURGE can protect them individually.

### 6.1 Layout and allocation

* Pages of fixed-size `RObj` slots.  Moving space: the pages of the current semi-space; non-moving
  space: its own pages with a free list (mark-sweep).
* Allocation: a bump pointer `top..end` in the current to-space page; the fast path is one compare
  and one add (it replaces marksweep's free-list pop: a load and a dependent store).  The slow path
  takes the next page from the pool, or collects.
* Trigger: allocated-since-GC > max(32 pages, 2 × live after the last GC) (heap ≈ 3× live, as
  marksweep's policy), or external memory over its limit (same `account_external` rule).

### 6.2 Collection

1. All pages of the moving space become from-space; to-space pages come from the pool (or are added).
2. Roots: `lang.visit_roots(&mut Forwarder)`, then the pending object's edges.  `visit(slot)`: if
   the referent is in from-space: forwarded → write the forwarding address; else copy the `RObj`
   (`ptr::copy_nonoverlapping`, 56 bytes) to to-space, set FORWARDED in the old header, store the
   new address in the old slot's first non-header word (the forwarding overlay), write the new
   reference.  If the referent is non-moving: set its mark bit and push it on a grey stack.
   `visit_ambiguous` is a compile-time refusal (`AMBIGUOUS_ROOTS = false`).
3. Cheney loop: a scan cursor over to-space pages calls `lang.scan_edges` on each copied object,
   interleaved with draining the non-moving grey stack, until both are exhausted.
4. `lang.sweep_weak` with `alive(slot)`: forwarded → rewrite and true; non-moving marked → true;
   from-space not forwarded → false.
5. `GcLang::moved(from, to)` for the objects whose flags ask for it (section 5), run during step 2/3
   at copy time so that side tables are rekeyed before their owners are scanned.
6. Finalization (6.3), then from-space pages go back to the pool (PURGE: to quarantine, `PROT_NONE`).
7. Non-moving pages are swept (unmarked → finalize, onto the free list).

### 6.3 Finalization in a copying collector

Payloads are Rust-owned, so a dead object must be finalized: a copying collector that never looks at
the dead would leak every `Vec`.  Options, in order of cost:

* sweep the from-space pages before releasing them (O(from-space), the cost marksweep pays anyway);
* skip the dead objects that own nothing (`owns_nothing` in `gc.rs`: small arrays, strings, floats,
  ranges — the bulk of short-lived objects) with a per-page count of "objects allocated with a payload"
  kept at allocation (one add on the slow path of payload-owning constructors, which already call the
  system allocator); a page whose count is zero is released without a walk.

ASTro does not need this (its payloads are GC objects); it is the price of Rust-owned payloads, and
the reason the gain of `copy` here is compaction and bump allocation rather than "touch only the live".
Moving payloads into the GC heap (a variable-size allocation API, `docs/gc.md` "What is not there yet")
removes it and is out of scope.

## 7. Backend `copy_gen` and the write barrier

`runtime/gc/copy_gen.rs`.  ASTro's §2.5 shape on pages: a nursery (two semi-spaces of 16 MiB of
pages), a tenured space (copying, major collections only), `PROMOTE_AGE = 3` (an age in the header,
incremented on each survival), a remembered set of tenured objects that may refer to the nursery
(a flat Vec with a header bit "remembered", overflow → heap walk), non-moving space as in `copy`
(always "old": its objects can be remembered).

* **Minor**: roots + remembered set → copy nursery survivors (age < 3 → nursery to-space, else
  tenured); a tenured object that still refers to the nursery after its scan stays remembered.
* **Major**: as `copy` over nursery + tenured.
* **Write barrier**: `Gc::store(holder, slot, v)` / `barrier_holder(holder)`: if holder is old and
  `v` young and holder not remembered → remember.  Hot path: two header-bit tests (holder old, holder
  not remembered) and a range test on `v`.

**Barrier insertion.**  Every store of a heap reference into a heap object (or into a payload it owns)
goes through `gc::store` or is followed by `gc::barrier_holder`.  Store sites by body kind: Array
elements (`AVec` writes: `ary_push`, `ary_set`, `splice`, sort in place, `replace`...), Hash entries
(`RHash` insert / update / default / default_proc / rehash), ivars (`IvBuf` writes, `ivar_set_raw`,
`gen_ivars`), `Range` (`range_set`), `Proc` / `Env` (closure variable writes — the hottest: `node_dasgn`
into a heap env), `Method` objects, `ClassData` tables (consts, cvars, mtbl owners, includes...),
every `RData` payload that holds Values (37 impls; e.g. `Enumerator`, `Struct`-like Data, `MatchData`,
fibers' `prev` / `resuming`), and the hidden Arrays used as tables.  Bulk writes (`Vec::extend`,
`clone`) use `barrier_holder` once.

**Audit (the wb_audit equivalent).**  ASTro made heap-pointer fields `const` under `ARO_GC_WB_AUDIT`
so a store that bypasses `ARO_STORE` is a compile error.  The Rust form: payload Value fields become a
`Member` type (`#[repr(transparent)] struct Member(Value)`) with `get()` and no `set` / `DerefMut`; the
only writer is `c.store(holder, &mut Member, v)` (or a bulk writer that calls `barrier_holder`).
Conversion is mechanical per body kind; the compiler lists every remaining direct write.  Runtime
cross-check (debug builds): at each minor collection, walk the old space and assert that every
old→young edge's holder is remembered (catches a barrier on the wrong holder).  ASTro's other layer,
PURGE, applies unchanged.  Cost: the barrier's two tests per store of a heap reference; measured on
array-heavy and closure-heavy benches before `copy_gen` is offered (the default build never has it:
the backend's `barrier` is empty and inlined away).

## 8. The blocker list (docs/gc.md), item by item

Precise roots:

| blocker | resolution | where |
|---|---|---|
| Values in Rust locals across allocations (cfunc signature, builtins, rt.rs) | discipline A/B/C/E (section 2), cfunc ABI `Args`, converted file by file under the gate | 2, 3, 9 |
| generated code keeps `@child` in Rust locals | `child_storage_*` overrides in `saruby_gen.rb`, feature `precise` until measured | 3 |
| `Block` / `Frame` on the machine stack | frame-chain walk; `Block`s reached from frames and Procs | 3 |
| value stacks scanned ambiguously (stale slots below `sp`) | nil-fill or per-frame extents; then `visit` instead of `visit_ambiguous` | 3 |
| Values copied into Rust heap buffers (`args.to_vec()`, `fiber_main`) | rewrite per site (2.4); inventory below | 2.4 |

Moving backend:

| blocker | resolution |
|---|---|
| `&'static mut` accessors in `object.rs` | already gone from converted files (heap_audit); the remaining call sites (`heap_audit.rb`, 498 at the fourth wave, fewer now) are converted before their files are declared precise. A body reference can then never outlive a GC point (borrow error). |
| addresses as identity | section 5 |
| raw class / object addresses in AST nodes | non-moving space for classes and literals; `ConstCache` slots visited (section 5) |
| `MethodEntry` / `Cref` visited through `*const` cast to `*mut` | visited as now (they are updated in place); fields become `Cell<Value>` so the write is sound |
| sweep runs `Drop` | finalize from-space as in 6.3 (cost noted); variable-size allocation out of scope |
| `Value` nil = 0x04, zeroed memory invalid | not a blocker: `alloc` takes a complete object (unchanged); to-space slots are written by copy |
| no store goes through the write barrier | `copy_gen` only: section 7 (Member type + audit) |

Inventory of hidden Value slots (statics, `RData` payloads, Rust buffers): see the trial README
(`~/ruby/src/trials/2026-11-25-astrust-moving-gc/README.md`, "Inventory"), produced by a read-only
audit of the tree at 83bb8bf; it is the work list of phase 1 and is kept there, not here, because it
shrinks with every converted file.

## 9. Migration order, cost, validation

1. **Phase 1a — the slice (this commit set).**  Precise zones + CHECK mode in the marksweep backend
   (section 11) and one converted builtin group, gated by STRESS + CHECK with the conservative scan off
   inside the zones.  Default build untouched (features off).
2. **Phase 1b — roots that cost nothing at run time**: statics and thread_locals into `visit_roots`;
   `RData::visit` completeness; frame-chain walk (visiting Blocks precisely, in addition to the stack
   scan); `MethodEntry` / `Cref` fields as Cells; `data_free_hooks` removed.  All in the default build:
   they only add work at GC time (a few hundred words).
3. **Phase 1c — cfunc ABI** `Args` (mechanical; the `with_args` copy disappears).  Measure (expected
   neutral or better).
4. **Phase 1d — convert builtins file by file** (the order of safe_heap.md's waves: array, hash,
   string, range/struct/enumerable, ...), each under the gate with its zone precise.  Values in Rust
   buffers are fixed in the same pass.
5. **Phase 1e — value stacks exact and node `@child` storage** (`saruby_gen.rb`), measured; under
   feature `precise` if over budget.  Then the main loop's zone becomes precise: the conservative scan
   is gone in precise builds (`AMBIGUOUS_ROOTS = false` for `Saruby` under `precise`).
6. **Phase 2 — `copy`** (section 6), non-moving space (section 4), identity fixes (section 5).  Gate:
   t/hand per-file outputs equal to the default build, STRESS + PURGE on the subset, reuse /
   aot_fork tests, AOT vs `--plain`.
7. **Phase 3 — `copy_gen`** (section 7) with the Member-type audit.

Expected hot-path costs: the cfunc ABI change and frame walk: none; `@child` slot storage: one store and
one load per child operand that is held across a sibling (to measure; koruby_precise pays it in every
build, so its numbers give no conservative baseline to compare with); value-stack nil-fill: per call, proportional to
temps; identity hash via object id: a flag test per identity hash; write barrier (copy_gen only): two
tests per heap store.

## 10. Fallback: mostly-copying (Bartlett)

If phase 1 stalls with a residue of unconverted code, `copy` can still run as a mostly-copying
collector: the residue's frames form conservative zones (section 11), objects designated by words in
those zones stay in place (their page is retained), everything else is copied.  Because conservative
zones can be discovered during tracing (a suspended fiber's stack), such a collector needs a mark pass
before evacuation (mark → evacuate → sweep), roughly twice the tracing of Cheney.  It is an intermediate
step only; the goal stays full moving with no stack-driven pinning.

## 11. Phase 1 slice: precise zones and CHECK (implemented)

* `gc::precise(c, |c| ...)` (sample/saruby/src/gc.rs) marks the frames it runs as a *precise zone*:
  `#[inline(never)]`, it spills the callee-saved registers into its own frame and records that address
  as a fence on a per-stack zone list.  `gc::conservative(c, |c| ...)` opens a conservative zone inside
  a precise one (converted code calling code that is not converted, e.g. `funcall`).  The ambiguous scan
  of the running stack (`GcLang::visit_roots`) covers only the conservative segments: from the stack
  top to the first fence, and from each `conservative` fence to the next `precise` one.  The register
  dump at each fence keeps the values the outer (conservative) frames had in callee-saved registers.
  Scanning less is the point: what converted code holds in Rust locals is not a root.
* CHECK (`make FEATURES=gc_check`): the backend is `MarkSweep<Saruby, true>`; freed slots are
  quarantined for `QUARANTINE` collections and poisoned by `GcLang::poison` (saruby: `klass` points at a
  `PROT_NONE` page; the body is `Free`).  The `false` instantiation (every other build) compiles the
  checks out.
* The converted group and its results: the trial README.
* Cost trap found while adding GC-time-only roots (session 3): `gc::alloc_with_by`'s `move || RObj {..}` closure is
  called on both branches of the allocation fast path.  When LLVM outlines it on the cold branch, the closure's captures
  must be in memory, and their stores get hoisted above the free-list test (5 stores per String literal: str +1.4 %),
  although the visitors that tipped the decision never run on that path.  The fix that goes with the root work: `#[inline(always)]` on that closure.
  Calling `make()` once before the branch (as MarkBitmap does) or passing the closure to a cold shim are both far worse
  (store-forwarding stalls; up to +28 %).  A change that only adds GC-time work can still move instruction counts by
  ±0.5 % through register allocation (closures, `make_env`): measure every root addition against the 21 benches.

## 12. Phase 1e: the value stacks exact, parks checked, Frame.blk, `sp` in a register

2026-11-26.  Design before implementation; numbers in the trial README, "Phase 1e".

### 12.1 Where a slot below `sp` can be stale

Every slot is a `Cell<Value>` and stacks start as NIL (`slots_new`, `nil_slots`), so a slot never holds a
word that is not a Value.  What makes the old scan ambiguous is staleness: a slot below `sp` that no one
wrote since it was last above `sp` holds a Value left by a popped frame, whose object may have been freed
(or, with a moving collector, moved) since.  Where `sp` covers slots before they are written:

* call staging: `stage(c, 2 + k)` then the receiver and the arguments are evaluated (GC points) and written
  one by one; `base[1]` (the callee's env slot) is written only by an entry node, never for a cfunc;
* `invoke_block_full` / `yield*`: `set_sp(base, 2 + argc)` after `put`, `base[1]` written by the entry;
* method / block entry: `enter` then nil-fill of `[2 + nparams, nlocals)` with no GC point in between (not stale).

Nil-filling staged areas at `stage` would cost 2 + k stores per call (fib: 3), which is over budget.

### 12.2 The invariant instead: every slot above `sp` is NIL after each collection

At the end of every collection's root scan, each context's slots above `sp` are reset to NIL.  Then at any
GC point every slot of a stack holds either NIL / an immediate, a Value that was below `sp` at the last
collection (so it was a root and is alive, or was updated), or a Value written since (an object allocated or
reachable since the last collection, which nothing could free meanwhile).  So `[0, sp)` is scanned with
`visit`, exactly: a stale slot only retains its object until the next collection, as the conservative scan did,
and a moving collector may update it (it is a valid Value).  Nothing on the hot path changes.

Cost, GC time only: per collection, the running and main contexts and each fiber that ran since the last
collection (a flag set on resume) clear their free part: stores for the first 8192 slots above `sp`, then
`madvise(MADV_DONTNEED)` over the rest of a `nil_slots` mapping (its pages read as NIL again: the mapping is a
private view of a NIL file, `fiber_os.rs`).  A stack not made by `nil_slots` (the fallback when the NIL file
cannot be made) is cleared by stores.  A stack taken back from the pool after a collection is cleared before
reuse (its first `WARM` bytes kept their old contents; `nil_slots_reset` gave back the rest).

What the clear also checks: R3 of stack_raw.rs ("sp is at or above every live slot").  Code that writes a slot
above `sp` and reads it after a GC point now reads NIL (under `gc_check`: a poison Value that faults when used;
the exact scan skips it).  `SARUBY_GC_STRESS` + `gc_check` over t/hand is the gate that finds such code.

### 12.3 Parks and yields (koruby's P1)

Audit (2026-11-26, master 0f77f68): every yield from Rust (`yield_block_raw`, `yield1_raw`, `call_proc_kw_raw`,
`invoke_block_full_raw`) puts the block's frame at `c.top()` and restores `sp` to that base, so it runs above
any park the caller opened; `node_yield` stages at `sp` too.  The one path that places a block frame at a base
taken before the builtin ran is the inline iterator of `node_call_blk` (`inline_iter.rs`): `bb = c.top()` at
the site, then `put_args` resets `sp` to `bb` at every step.  The `*_loop` functions it runs park nothing today
(their state is in Rust locals), so P1 does not happen; a loop converted to park its state would have its park
overwritten by the block's frame.  Check, `gc_check` builds: `park` records its slots' end per stack, and every
move of `sp` downward (`set_sp`, `sp_reset`, `at_entry`) asserts that it stays at or above the innermost live
park of that stack.  This is deterministic (no stress needed) and fires on the koruby P1 pattern.

### 12.4 Frame.blk and Block values (from phase 1b)

Implemented on its own branch (`moving-gc-e1-blk`, independent of 12.2).  A Block a frame points at is either a
Proc payload (visited through the Proc) or a Rust-stack temporary of its caller.  Both are now also visited through
the frame chain of every context (`Frame.blk`, `FrameFull.self_blk`; `visit_ctx`, `visit_main_ctx`).  A moving
collector updates them through the shared pointers frames and callers hold, so Block's Value fields (`self_`,
`env`, `proc_val`, `defined_class`) are `Cell<Value>` and BlockKind's Values (Method, Cfunc data, Proc) moved into
one `data` Cell, as MethodEntry's Bmethod Proc did (f203599).  Block is Clone, not Copy; its clone is a bitwise copy
(one `unsafe`): field by field, LLVM copied the 88 bytes with a dozen narrow moves and closures rose 2.3 % / 4.0 %
instructions.  `block_to_proc`'s write of `proc_val` through a `*const Block` cast to `*mut` became a Cell set.

### 12.5 `Ctx.sp` in a register: measured, not threaded yet

The user's rule (2026-11-26): thread `sp` only for a clear per-benchmark gain on the call-heavy benches at
codegen-units = 1 on sp4; otherwise keep `Ctx.sp`.  Measured on master 0f77f68, sp4 (trial README "Phase 1e"):

* Interpreter (`--plain`): per call, node_call loads `sp` once and stores it at `stage` and at `set_sp`; the method
  entry does one read-modify-write (`sub %r, (ctx)`).  Instruction samples on those instructions: fib 1.00 %,
  method_call 1.40 %, tak 1.58 %, binary_trees 1.81 %, closures 0.17 %, iterators 0.09 % (skid puts up to 5 % more on
  the next instruction).
* Baked code: method_call's loop (about 53 instructions per call, callee compiled in) does 3 `Ctx.sp` accesses per
  call (load at the stage, store at the stage, store at `set_sp`), with no call or GC point between them on the fast
  path.  Samples: method_call 2.65 %, tak 2.44 %, iterators 2.01 %, binary_trees 1.55 %, closures 0.92 %, fib 0.32 %
  (+0.8 - 6 % on the next instruction).
* So threading could remove at most about 3 instructions per call on the baked fast path (about 5.7 % of
  method_call's), and about 1 - 2 % of the interpreter's.  Getting there is not local: the stores can only move to the
  slow paths if every EVAL and every out-of-line call of the generated code carries `sp`, and only the allocation /
  GC-point entries write it into `Ctx` (koruby's "only korb_alloc writes sp_top", enforced by a type: an `Sp` token the
  callee consumes).  That is an ABI change of the generator (`saruby_gen.rb`, every node), not a patch of one path.

Decision for now: `Ctx.sp` stays.  The prototype was not built in this session; the bound above says it is worth
building as its own item (todo 1.5), and the decision is to be made from its sp4 numbers per benchmark.

### 12.6 What still relies on the machine-stack scan

With the value stacks exact, the remaining conservative roots are the machine stacks (the running one and suspended
fibers').  `make FEATURES=gc_check` + `SARUBY_GC_PRECISE_ALL=1` scans neither; with `SARUBY_GC_STRESS=1` what breaks
is what still keeps a Value only in a Rust local: generated node code's `@child` values (phase 1e, 12.7), cfuncs on
the old ABI (phase 1c / 1d), and helpers in rt.rs / vm.rs.  The default build keeps the scan.

### 12.7 `@child` values in slots: cost estimated, under feature `precise` only

`a + b` keeps `a` in a register while `b` (a call) runs; a moving collector needs `a` in a slot.  Storing every
earlier child of a multi-child node in a frame or value-stack slot costs a store and a load per such child: in
fib's `fib(n - 1) + fib(n - 2)` and `n < 2` alone that is about 10 of the ~84 instructions per call of the baked
code (an estimate from the node shapes, not measured), far over 0.3 %.  So it goes under cargo feature `precise` (with `GC=copy`), as section 9 planned; a test that
skips the store for immediates (`if a.is_heap()`) still costs a test and a branch per child.  Not implemented yet.
