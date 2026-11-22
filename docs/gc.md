# ASTrust GC framework (`runtime/gc`)

The Rust counterpart of ASTro's `runtime/precise_gc`: the collector is split into a
language binding and a backend, joined at compile time.

```
runtime/gc/mod.rs          traits GcLang / GcBackend / EdgeVisitor / RootVisitor, Gc<L, B>, GcHeader
runtime/gc/pages.rs        fixed-size slot pages + "which slot does this word point into"
runtime/gc/stack.rs        ambiguous roots from machine stacks (register dump, stack words)
runtime/gc/mark_sweep.rs   backend "marksweep": mark/free bits in the object header, free list
runtime/gc/mark_bitmap.rs  backend "bitmap":    mark/live bitmaps per page, header untouched
sample/saruby/src/gc.rs    saruby's binding (the only user so far)
```

An embedder includes the directory as a module, like `runtime/astrust.rs`:

```rust
#[path = "../../../runtime/gc/mod.rs"]
pub mod astrust_gc;
```

## The interface

```rust
pub struct GcHeader(/* one u32, owned by the backend */);

pub unsafe trait GcLang: Sized {
    type Ref: Copy;                      // a reference word (tagged pointer, NaN box, ...)
    type Obj;                            // the heap object: one fixed-size slot
    const HEADER_OFFSET: usize;          // where Obj keeps its GcHeader
    const AMBIGUOUS_ROOTS: bool;         // visit_roots calls visit_ambiguous
    fn as_object(r: Self::Ref) -> Option<NonNull<Self::Obj>>;    // "is this word a heap reference"
    fn from_object(p: NonNull<Self::Obj>) -> Self::Ref;          // how a moving backend rewrites a slot
    fn visit_roots<V: RootVisitor<Self>>(&mut self, v: &mut V);
    fn scan_edges<V: RootVisitor<Self>>(&mut self, obj: &mut Self::Obj, v: &mut V);  // + words it owns
    unsafe fn finalize(&mut self, obj: NonNull<Self::Obj>);      // a dead object: release what it owns
}

pub trait EdgeVisitor<L: GcLang> { fn visit(&mut self, slot: &mut L::Ref); }
pub trait RootVisitor<L: GcLang>: EdgeVisitor<L> { fn visit_ambiguous(&mut self, words: &[usize]); }

pub trait GcBackend<L: GcLang>: Sized {
    const NAME: &'static str;
    const MOVING: bool;
    const AMBIGUOUS_ROOTS: bool;         // can pin what ambiguous words designate
    fn new() -> Self;
    fn alloc(&mut self, lang: &mut L, ctl: &mut Control, obj: L::Obj) -> NonNull<L::Obj>;
    fn collect(&mut self, lang: &mut L, pending: Option<&mut L::Obj>);
    fn barrier(&mut self, holder: NonNull<L::Obj>, v: L::Ref) {}          // default: none
    fn barrier_holder(&mut self, holder: NonNull<L::Obj>) {}
    fn pin(&mut self, obj: NonNull<L::Obj>) {}
    fn account_external(&mut self, bytes: usize) {}
    fn each_object<F: FnMut(NonNull<L::Obj>)>(&mut self, f: F);
    fn stats(&self) -> HeapStats;        // slots, free, allocated, freed
}

pub struct Gc<L: GcLang, B: GcBackend<L>> { pub ctl: Control, /* backend */ }
impl Gc<L, B> {
    fn new() -> Self;
    fn alloc(&mut self, lang: &mut L, obj: L::Obj) -> NonNull<L::Obj>;   // inline; may collect
    fn collect(&mut self, lang: &mut L);
    fn store(&mut self, holder: NonNull<L::Obj>, slot: &mut L::Ref, v: L::Ref);   // write barrier
    fn barrier_holder(&mut self, holder: NonNull<L::Obj>);
    fn pin / account_external / each_object / stats / backend_name
    fn set_stress(&mut self, n: usize);  fn set_disabled(&mut self, d: bool) -> bool;
}
```

Points of the design:

* **Static dispatch.** `Gc<L, B>` is a concrete type per build (saruby: `type Backend = ...` under a
  cargo feature).  `Gc::alloc` and `Gc::store` are `#[inline(always)]`, and a backend's `alloc`
  is expected to be: the common case of an allocation is inlined into the caller (and into baked
  AOT code — so an embedder's build id must cover the backend, see `sample/saruby/build.rs`).
* **Edges are slots.** Every visit is `visit(&mut Ref)`.  Marking only reads the slot; a moving
  backend forwards the object and writes `L::from_object(new)` back.  One enumeration
  (`visit_roots`, `scan_edges`) serves mark, forward and pointer update.
* **Ambiguous roots are explicit.** `RootVisitor::visit_ambiguous(&[usize])` takes words that may
  or may not be references (interior pointers included).  Backends must keep and never move what
  they designate, and never rewrite the words.  A binding declares `AMBIGUOUS_ROOTS = true` if it
  calls it, a backend declares whether it can honour it, and `Gc::new` refuses the combination
  "binding uses them, backend cannot" at compile time.  A precise binding simply never calls it.
  `scan_edges` gets a `RootVisitor` too: an object may own ambiguous words (a suspended fiber's
  machine stack), which are then traced only while the object is reachable.
* **The object being allocated is a root.** `alloc` takes the object by value.  If a collection
  runs first, the backend receives it as `pending` and scans its edges, so values stored in the
  new object need no other root during the call (and a moving backend updates them).
* **Stress mode belongs to the framework.** `set_stress(n)` collects at every n-th allocation,
  through the same `Control::collect` every backend uses.  `Control` also holds GC.disable
  (backends ask `ctl.disabled()` before an allocation-driven collection), the collection count
  and the time spent.
* **Write barrier.** `Gc::store(holder, slot, v)` writes the slot and calls the backend's
  `barrier`; `barrier_holder` is the barrier alone, for payloads written in bulk.  Both backends
  have an empty barrier and **nothing calls `store` yet** (saruby exposes it as `gc::store`).  A
  generational or incremental backend makes it mandatory for every store of a reference into a
  heap object; inserting those calls is future work.
* **What is not there yet.** One fixed-size `Obj` per object: there is no variable-size or
  byte allocation, so payloads stay Rust-owned (`Vec`, `Box`) and are released by `finalize`.
  No weak references or ephemerons.  One heap per process is assumed by nothing in the framework,
  but saruby keeps its heap in a global.

## Writing a backend

Implement `GcBackend<L>` for a type generic over `L: GcLang` (see `mark_sweep.rs`, about 200 lines):

1. `alloc`: write `obj` into a slot and return it.  Keep the fast path small and
   `#[inline(always)]`; move everything else to an `#[inline(never)]` function that takes and
   returns the object by value (taking its address on the fast path would force it into memory).
   When a collection is due and `!ctl.disabled()`, call `ctl.collect(self, lang, Some(&mut obj))`
   — never `self.collect` directly, so the count, the timer and stress mode stay in one place.
2. `collect(lang, pending)`: call `lang.visit_roots(&mut visitor)`, scan `pending` if present,
   then `lang.scan_edges(obj, &mut visitor)` for each object reached, until nothing is left.
   Call `unsafe { lang.finalize(p) }` exactly once for every object not reached, before its
   slot is reused.
3. The visitor implements `EdgeVisitor<L>` (`L::as_object(*slot)` says whether the slot holds a
   reference) and `RootVisitor<L>`.  If the backend cannot pin, set `AMBIGUOUS_ROOTS = false`.
4. Per-object state goes in the `GcHeader` at `L::HEADER_OFFSET` (private accessors in `mod.rs`)
   or in a side table (`mark_bitmap.rs`).  A fresh object's header is 0.
5. A moving backend additionally: rewrites every visited slot, honours `pin`, and must not move
   objects designated by ambiguous words.  It needs `finalize`-free relocation of `Obj`
   (a plain memory copy; `Obj` must not be self-referential).

`pages.rs` gives slot-based backends their pages and the conservative lookup
(`Pages::resolve(word)`); `R` bytes at the start of each page can be reserved for side tables.

Add the backend to the embedder's type alias (saruby: `sample/saruby/src/gc.rs`, a cargo feature
`gc-<name>` in `Cargo.toml`, `make GC=<name>`).

## Writing a binding (a new language)

1. Put a `GcHeader` in the object type and build objects with `GcHeader::new()`.
2. `unsafe impl GcLang`.  The obligations (why the trait is unsafe): `HEADER_OFFSET` is right,
   `Obj` is at least 8-aligned, and `as_object` returns only pointers that came from `Gc::alloc`
   of this heap.
3. `visit_roots`: visit every slot that can hold a reference and is not inside a heap object.
4. `scan_edges`: visit every reference slot of one object, including those inside payloads the
   object owns (`Vec<Ref>`, hash tables).  It is called on live objects and on the pending one.
5. `finalize`: drop what the object owns.  It must not allocate or look at other objects (they
   may already be finalized).
6. If references can sit in native locals, pass the machine stack as ambiguous roots:
   `stack::with_current_stack(top, |words| v.visit_ambiguous(words))`, and
   `stack::words(lo, hi)` for suspended stacks (fibers): from `scan_edges` of the object that
   owns the stack, so a suspended stack nothing refers to is not a root.

## What "precise" requires of the embedder

A precise binding has `AMBIGUOUS_ROOTS = false`: every reference the program still needs is in a
slot that `visit_roots` or `scan_edges` reports.  With a tree-walking evaluator that means, as in
ASTro (no stack maps):

* **Values live in value-stack slots.** Locals, temporaries and staged arguments are slots of a
  stack that `visit_roots` walks exactly.  The generator hook `child_storage_stmt` /
  `child_storage_expr` (README, "@child storage") moves `@child` operands from Rust locals to
  such slots.
* **No raw reference across an allocation.** Native code must not keep a `Ref` (or a pointer or
  `&mut` into an object's payload) in a local across any call that can allocate; it re-reads the
  slot afterwards.  With a moving backend the old value is not merely unrooted, it is wrong.
* **Stress mode is the test.** `set_stress(1)` makes every allocation a collection; a moving
  backend that also unmaps from-space turns each violation into a fault.

Soundness of the Rust API: every function of `Gc` is safe, and the framework itself never
dereferences anything but what `GcLang` hands it.  What cannot be expressed is liveness: the
`NonNull<Obj>` returned by `alloc` (saruby: a `Value`) is valid only while the object is
reachable from the roots, and dereferencing it is the embedder's `unsafe`.  Today a `Value` held
only in a Rust local stays alive because saruby reports the machine stack as an ambiguous root;
nothing in the type system says so.

## saruby's binding today

`sample/saruby/src/gc.rs`: `type Saruby = Ctx` (the allocating context; the other roots are globals), `type Heap =
Gc<Saruby, Backend>`, `gc::alloc(c, klass, body)` (through `Ctx::alloc`), `gc::collect(c)`, `gc::store`, `gc::each_object`.

* Roots (`vm::visit_roots`, `vm::visit_ctx`): exact — the `Vm` fields (builtin classes,
  globals, `node_values`, `pinned` (the current fiber among them), `frozen_lits`,
  `symproc_cache`, crefs), the main `Ctx`'s `esc_val` / `errinfo` / `last_match` /
  `esc_tag_v`; ambiguous — the running machine stack with callee-saved registers and the main
  value stack.
* A Fiber is an ordinary object: `FiberData::visit` (fiber.rs) traces its `Ctx` (value stack,
  ambiguous) and, while it is switched out, its machine stack from the saved stack pointer
  (ambiguous; the main thread's root fiber owns the main stack).  A suspended fiber nothing
  refers to is collected; its stacks go back to pools (fiber_os.rs).
* Edges (`GcLang::scan_edges`): the match over `Body`, `vm::visit_class` / `visit_me` /
  `visit_block`, and `RData::visit(&mut self, &mut gc::Visitor)` for native payloads
  (`gc::Visitor` is `dyn EdgeVisitor<Saruby>`: one virtual call per edge of a Data object).
* Finalization: generic ivars and `data_free_hooks`, then `drop_in_place` of the body; the slot
  is left holding `Body::Free` so a stale reference does not find a dropped payload.
* Backend: `make` (marksweep) or `make GC=bitmap`.  `SARUBY_GC_STRESS=n` arms stress mode once
  the VM has booted.

State under `SARUBY_GC_STRESS=1` (t/hand `gc* array* hash* str*`, 246 files, output compared
with a run without stress, 2026-11-20): marksweep 1 file differs (`gc_remset_overflow_1030.rb`,
a timeout); bitmap 4 (`gc_remset_overflow_1030.rb` timeout, `string_misc_compat_2026_09_11.rb`,
`string_partition_to_str.rb`, and a SEGV in `stringio.rb` at `sio_readlines`).  The bitmap
backend reuses the lowest free slot first, so a stale reference aliases a new object sooner;
whether the three extra files are rooting bugs in saruby shown by that order or a fault in the
backend has not been determined.

### What still blocks precise roots or a moving backend

Each item is an assumption in saruby's code, not in the framework (see also
`sample/saruby/docs/roadmap.md` section 4).

Blocking precise roots (dropping the ambiguous machine-stack scan):

* Values in Rust locals across allocations, everywhere: the cfunc signature
  `fn(&mut Ctx, Value, &[Value], *const Block)` (`src/vm.rs`, `CFunc`), all of `src/builtins/`,
  the runtime helpers in `src/rt.rs`.
* Generated code keeps `@child` values in Rust locals (`lib/astrustgen.rb`
  `child_storage_stmt` default; saruby does not override it in `saruby_gen.rb`).
* `Block` and `Frame` structs live on the machine stack and hold Values (`self_`, `env`,
  `proc_val`, `defined_class`: `src/vm.rs` `struct Block`); they are reached only by the
  ambiguous scan, and `Block.outer_blk` points at such a stack copy (`vm::visit_block`).
* The VM value stacks are scanned as ambiguous words (`vm::visit_ctx`): slots below `sp` that
  no frame wrote yet hold stale Values.  They hold only Values, so this one can become exact
  once frames clear or bound their slots.
* Values copied into Rust heap buffers across calls (`args.to_vec()` in `src/vm.rs`; in
  `src/fiber.rs` `fiber_main` the Array they came from is kept on the stack), which no scan sees.

Blocking a moving backend (in addition):

* `&'static mut` borrows into bodies: the accessors in `src/object.rs` (`obj`, `rary`, `rstr`,
  `rhash`, ...).  Array methods no longer use them: `sample/saruby/docs/safe_heap.md`.
* Addresses as identity: `gen_ivars` keyed by address (`src/vm.rs` `ivar_set_raw`, and
  `gc.rs` `visit_gen_ivars` / `remove_gen_ivars`), `object_ids` keyed by address
  (`vm::object_id`), `Object#hash` and identity hashes from the raw word (`src/hash.rs`
  `key_hash`: hash tables would need a rehash after a move), `data_free_hooks` taking
  `*mut RObj`, `recursive_guard` (`src/builtins/kernel.rs`).
* Raw class and object addresses in AST nodes: `CallCache.klass`, `IvarCache.klass`,
  `ConstCache.val` (`src/vm.rs`), literal `Value` operands (kept alive by `Vm::node_values`, but
  the node's copy would not be updated).  Classes would be pinned (`Gc::pin`) or caches flushed.
* `MethodEntry`, `Cref` are `Box`ed outside the heap and hold Values: they are visited through
  `*const` pointers cast to `*mut` (`vm::visit_me`, `vm::visit_roots`).
* Sweep runs `Drop` (`GcLang::finalize`): payloads are `Vec` / `Box` / `HashMap`.  Moving the
  fixed-size slot is possible, but the gain (bump allocation, compaction) needs payloads
  allocated by the collector — the missing variable-size allocation API.
* `Value` is `NonZeroU64` with nil = 0x04 (`src/value.rs`): zero-filled memory is not a valid
  object, so a backend cannot hand out zeroed slots for later initialization (`alloc` takes a
  complete object instead).
* No store goes through the write barrier (`gc::store`): required before any generational
  backend.
