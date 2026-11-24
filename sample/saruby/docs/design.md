# saruby (ASTrust) — design notes

saruby is a Ruby implementation in Rust built on ASTrust (`../../lib/astrustgen.rb`,
`../../runtime/astrust.rs`), the Rust port of ASTro.  It is a rewrite of
`astro/sample/koruby_precise` (C) — same goals (CRuby drop-in, AOT by partial evaluation),
different implementation language.

## Pipeline

```
source --prism (C, vendor/prism) --> serialized AST --src/prism.rs + prism_ast.rs--> Rust AST
       --src/compile.rs--> ASTrust nodes (node.def) --interpreter: DISPATCH_* (node_dispatch.rs)
                                                    \-AOT: SPECIALIZE_* emit Rust --rustc--> code_store/all.so
```

* `node.def` — node kinds with Rust bodies.  `ruby saruby_gen.rb` (via `make`) generates
  `src/node_*.rs`.  Never edit generated files.
* Every dispatcher is `fn(c: &mut Ctx, n: NodePtr, fp: Fp) -> RV` where
  `RV = Result<Value, Esc>`.  `Esc` is a zero-sized error; the kind/payload of a non-local exit
  (raise, break, next, return, throw, ...) is in `c.esc`, `c.esc_val`, `c.esc_tag`.
* `fp` = current frame on the VM value stack: `fp[0]` = self, `fp[1]` = env
  (heap Env holding variables captured by blocks, or the outer env), then parameters/locals.
  Slots are `Cell<Value>`: `fp[0].get()`, `slot(fp, i).set(v)`.
* Calls stage `[recv, -, args...]` at sp (`stage(c, n)` gives a `Stk`) and the callee frame
  starts there (no copying).
* `Fp`, `Stk` and `c.sp` belong to the stack backend, chosen at build time: `src/stack_raw.rs`
  (default: raw pointers, all of the value stack's `unsafe`) or `src/stack.rs`
  (the default; `make FEATURES=raw_stack` swaps in the raw one: every access checked, `#![forbid(unsafe_code)]`).  Code outside
  them uses only their common API.  See docs/safe_stack.md.

## Values (src/value.rs)
Fixnum (`...1`), Flonum (`..10`, CRuby encoding), heap pointer (`.000`), nil=0x04, false=0x0c,
true=0x14, undef=0x1c, Symbol `(id<<8)|0x34`.  No value is 0 → `RV` fits in one register.

## Objects (src/object.rs) and GC (src/gc.rs)
One fixed-size slot per object (`RObj { klass, flags, gc, body: Body }`); variable data in Rust
Vecs/Boxes owned by the slot.  Non-moving mark & sweep, as a backend of the ASTrust GC framework
(`../../runtime/gc`, `../../docs/gc.md`); `src/gc.rs` is saruby's binding.  `make GC=bitmap`
builds with the other backend.  Native payloads report their Values with
`RData::visit(&mut self, v: &mut gc::Visitor)` (`v.visit(&mut self.field)`).
One fixed-size slot per object (`RObj { klass, flags, body: Body }`, 56 bytes, asserted in
object.rs); variable data in Rust Vecs/Boxes owned by the slot.  Non-moving mark & sweep.

* Instance variables (`src/ivbuf.rs`, no unsafe): a class numbers its ivars
  (`ClassData.ivar_index`); slot i of every instance is that ivar, `UNDEF` when unset.  Slots
  0..3 are in the GC slot, 3..8 in one block made with the 4th, the rest in that block's Vec.
  `new_object` makes as many slots as the class has seen, so the site cache `{class, index}`
  (`IvarCache`, attr reader/writer cache kinds) is one test for a read and for a write: there is
  no append and no per-object shape.  Consequence kept from before: `instance_variables` lists
  in the class's order, not the object's assignment order.
* Short strings live in the slot too (`src/sbuf.rs`).
* So do Arrays of up to 3 elements (`src/avec.rs`, no unsafe): `AVec` is `Inline(len, [Value; 3])`
  or `Heap(Vec)`, derefs to `[Value]`, and an array stays out of line once it needed a general Vec
  operation.  Every element access pays the Inline/Heap test (one compare and branch; the Heap path
  falls through).  Sweep skips `Drop` for bodies that own nothing (`gc.rs`, `owns_nothing`).

* Env (`src/envvals.rs`, no unsafe): the first two values are in the slot, the rest in one boxed
  Vec; an Env of up to two values needs no malloc and its sweep frees nothing.  Baked code indexes it
  with a constant, so an access is one load.

**Rooting (important):** heap objects are traced precisely; the machine stack and registers
are scanned conservatively, and the VM value stack too.  So Values in Rust locals are safe.
**Never keep a Value only inside a Rust heap buffer (Vec<Value>, Box) across an allocation or
a Ruby call** — if needed, put it in a Ruby Array (`ary_new_from`) held in a local.  Don't hold
`&mut` borrows into an object (e.g. `rary(v)`) across calls that can run Ruby code; iterate by
index and re-fetch (`ary_get(v, i)` with `ary_len(v)` checked each iteration).
In converted modules (array.rs, builtins/array_methods.rs) the last rule is a compile error
instead of a convention: bodies are reached through the context (`c.ary(v)`, `c.ary_mut(v)`,
`src/heap.rs`), and a borrow of `c` cannot be held across a call that takes `c`.  New code
uses that API; see docs/safe_heap.md (design, recipe, what is left).

## Methods
* Builtins are `CFunc = fn(&mut Ctx, self: Value, args: &[Value], blk: *const Block) -> RV`,
  registered with `define_cfunc(klass, "name", f, arity)` (arity -1 = variadic; checked).
  `args` comes from `with_args`: the staged slots themselves, or a copy of them with the checked stack.
  Keyword args to a cfunc arrive as a trailing Hash.  Whether the caller passed keywords is a bit of
  the cfunc's frame (`FR_KW`, set by `invoke_cfunc` from the call flag `CF_KW`, as CRuby's
  `VM_FRAME_FLAG_CFRAME_KW`): `kw_given(c)` is `rb_keyword_given_p()`, `kw_arg(c, args)` the Hash,
  `kw_split(c, args)` = rb_scan_args' `":"`, `kw_unknown` rb_get_kwargs' check.  A positional Hash
  (ruby2_keywords-flagged or not) is never keywords.  A Rust helper reading it must run in the
  cfunc's own frame with the cfunc's own `args` (or a slice ending at the same last argument).
  To pass keywords on: `funcall_kw(.., kw_given(c))`, `call_proc_kw`, `yield_with_self_kw`.
* Call Ruby from Rust: `funcall(c, recv, sym, &args, blk)`, `yield1(c, blk, v)`,
  `yield_block(c, blk, &args)`, `call_proc(c, proc, &args, blk)`.
* Raise: `return err(c, vm().c_argument_error, "msg")`, `arg_err`, `type_err`, or
  `Err(raise_exc(c, exc_obj))`.
* Ruby-level built-ins live in `prelude/core.rb` (loaded at boot). build.rs parses it with prism
  (`tools/prism_serialize.c`, options from `src/prism_opts.rs`) and embeds the serialization, so
  boot only reads and compiles it (`prism::read_serialized`, `compile::compile_parsed`).

## Calls, frames and blocks (the fast paths)
Measurements and what was tried: `~/ruby/src/trials/2026-11-21-saruby-call-path/` and `-call-path-2/`.

* A call site (`node_call`, `node_call_blk`, `node_attrasgn`) stages `[recv, -, args...]`, stores its
  line in the caller's frame and tests its cache: receiver class and `method_serial()` (a static
  counter, bumped by `bump_method_serial()` whenever a method table, a visibility or the hierarchy
  changes).  Baked code keeps one path inline, chosen by the cache at bake time (`site_kind`): the
  callee compiled into the site (`callee@spec`), or the site's entry / attr / builtin path
  (`site_fast`); everything else is `call_cached_out`.  A compiled-in callee is guarded by
  `CallCache.spec_klass` instead of `klass`: it is set by `call_spec_out` (the site's out-of-line
  call, the only thing that refills its cache) when the entry the cache now holds has the baked hash,
  so the per-call test is heap, class, serial.
* A compiled-in entry gets its method entry and block as `CallArgs`, a `@specin` operand: in
  registers through `EVAL_SPEC!`; called through a dispatcher it reads `c.call_me` / `c.call_blk`.
* A compiled-in self call without arguments of a method without parameters, locals, env or block
  literal (`spec_shared`, baked as `shared`) runs the callee in the caller's frame: no staging, no
  frame check, no sp change; the entry (`CallArgs.shared`) skips `enter` and the env slot.
* A call site inside its own callee (recursion: the target is being specialized) is baked with
  `R_<hash>`: a direct call of the target's entry `S_<hash>::run` (CallArgs as a two-word `CallPass`,
  the native stack check before it), under the compiled-in callee's guard; `SD_<hash>` of such a target
  only passes `CallArgs::load(c)` on.  A small target is first compiled into its own sites once
  (`specialize_open`), so every other level is inline; `ASTRUST_REC_UNROLL=0` at bake time turns that off.
  Measurements: `~/ruby/src/trials/2026-11-21-saruby-recursion/`.
* A method with only required parameters has a `node_method_entry` (`Iseq.entry`): it expects sp
  just above the staged arguments (where `stage` left it), makes the 6-word `Frame` on the native
  stack, nil-fills, links `c.frame`, runs the body and consumes its `return`.  Its flags (`ME_*`,
  set in `compile.rs`):
  * `ME_ID`: the frame gets a fresh id.  Only bodies with a block literal or an eval-like call
    (`eval`, `binding`, `send`, `method`, ...) need one, as the target of a `return` inside a
    block.  Other frames have id 0; their own `return` carries tag 0 and is consumed by the
    innermost entry, which is theirs.
  * `ME_LEAF`: the body only reads (nil / true / false / self / literal / local / ivar, and `if`,
    `&&`, `||` of those).  It cannot call, raise or look at a frame, so the entry is just the body:
    no `Frame`, no checks, no sp change.  Keep such node bodies free of `c.frame`.
  * `ME_ENV`: locals captured by blocks live in a heap env.
  * `ME_NOCALL`: the body has no call site (operators call only on their slow path).  Compiled into
    its caller (`CallArgs.inl`, a constant there), the entry restores `c.frame` from the value it read,
    not by reloading `fr.prev`: that reload put a store-load round trip on every call of a leaf callee
    (`~/ruby/src/trials/2026-11-21-saruby-call-path-3/`).  Out of line the value would need a saved register.
* Keyword methods whose parameters are lead + keywords in declared order (nothing else) have two more
  entries over the same frame layout: `Iseq.kentry` takes lead + every keyword as plain parameters,
  `Iseq.dentry` (no required keywords) takes lead only and runs the defaults first.  A literal keyword
  site (`node_call_kw`) checks once per cache fill that it passes exactly the callee's keywords in
  order (`kw_dense_fill`; `CallCache.fast_argc = KW_DENSE | argc`) and then calls the kentry like any
  entry; baked, the kentry is compiled into the site.  dentry is the cache's entry for calls without
  keywords (`Iseq::plain_entry`).  dentry has its own copy of the body, so it is not made where a node
  caches a literal object (`has_lit_cache`).  A body under a kentry is baked separately only if it
  also ran through `invoke_iseq` (`body_used`).
* The native stack check of a call is made where an entry is called through its dispatcher
  (`run_entry`, `site_fast`), not in the entry: an entry compiled into its call site adds no native
  frame, and every recursion passes a dispatcher call.
* A block with only required parameters has a `node_block_entry` (`Iseq.bentry`), the block's baked
  function.  `run_block_fast` (yield from a builtin: `yield1`, `yield_block`; from Ruby:
  `node_yield`) puts the `Block` in `c.call_blk`, the arguments at `base[2..]` and dispatches it.
  A `->` literal's entry has `ME_LAMBDA` (lambda frame: own return target, catches its `break`);
  `call_proc_kw` runs it for a call with exactly its parameter count, no keywords and no block.
  Other ways to run a block (other lambda calls, `define_method`, one Array spread over several
  parameters) set up the frame in `run_block_iseq` and dispatch `Iseq.body`; that body is baked
  separately only for blocks that ran this way before the bake (`Iseq.body_used`).
* A Ruby `yield` site records the block iseq it ran (`YieldCache.seen`).  When it saw one, the bake
  compiles that block's entry into the site (`spec_block`, parameter count baked as `np`); the run
  compares the block's iseq with `YieldCache.fast` (set by `srb_yield_miss` once that iseq's entry
  hash equals the baked one) and that the block is not a lambda proc.
* **Inline iterators** (`src/inline_iter.rs`).  A `node_call_blk` whose cache held one of the builtin
  iterators (`Integer#times` / `upto` / `downto` / `step`, `Range#each` / `map`, `Array#each` / `map` /
  `select` / `reject` / `inject` / `each_with_index`, `Hash#each`; identified by method entry as for
  inline builtins, `CallCache.bi`, only for the class itself) is baked with the iterator's number and the
  block's parameter count (`it`) and the block's entry compiled in (`body@spec`).  The site runs the loop
  (`iter_begin` / `iter_next` / `iter_step` / `iter_end`) under the builtin's cfunc frame (backtraces
  unchanged) and calls the entry directly each step.  A block without env (`IT_SHARE`) gets its frame,
  self and env once per loop (`block_frame`; the entry with `CallArgs.shared` skips them), the line reset
  per step.  Receivers or arguments the loop does not handle (Bignum, Float limit, endless Range, another
  class) take the call out of line; an empty or stale cache is filled first (`iter_refill`) so a loop
  run once at top level is inline too.  `SARUBY_NO_IT=1` at bake time turns it off.

* `recv.send(:name, args...)` / `__send__` at a site whose cache holds the builtin (`CallCache.bi =
  BI_SEND`, baked as `SITE_SEND`) is the call of `name` itself (`send_fast`): the name is looked up
  in a direct-mapped (class, name, method serial) table (`Vm.send_cache`), the arguments are shifted
  down over it on the staged frame and the method runs there, with no frame for `send` (CRuby lists
  none).  String names, refinements and missing methods take the builtin.  `public_send` keeps its
  cfunc frame (CRuby lists it).  `f(*a)` with only a splat stages `a`'s elements directly
  (`srb_call_splat1`, no argument Array).
* Operator nodes (`node_opt_*`, `@x += v`, `!x`) keep their call cache behind a pointer (`OpCc`),
  made when the site first calls a method.

## Tracing (TracePoint, set_trace_func; builtins/tracepoint.rs)
Nothing on a fast path looks at tracing; `vm::trace_mask()` (the union of the enabled hooks' events) is
read only on paths that are slow anyway.  Measurements: `~/ruby/src/trials/2026-11-22-saruby-converter-tracepoint/`.

* **Calls.** While call events are traced, `fill_cc` leaves an entry that never hits (serial 0), and
  enabling / disabling bumps the method serial.  Every inline path of interpreted and baked call sites
  (entries, compiled-in callees, inline builtins and iterators, attr, `send`, `Klass.new`) is guarded by
  the serial, so all calls go out of line to `invoke_impl`, which hands them to `invoke_traced`.
* **Code compiled for tracing.** Once a source names `TracePoint` or `set_trace_func` (or a hook is made),
  later code is compiled with `vm::TRACE_COMPILE`: a `node_trace` before each statement (one per line, as
  CRuby's newline nodes) and rescue clause, operators as plain calls, no entry nodes (so calls and yields
  take `invoke_iseq` / `run_block_iseq`, which report call / return events in the callee's frame), and
  every local in the env (`TracePoint#binding`).  Other programs compile and bake as before.
* Code compiled earlier (the prelude, libraries loaded first) reports call-level events only; prelude
  methods are reported as builtins (`c_call`), and calls they make are not reported.

## Lexical scope (Cref) and refinements
* A frame's `cref` is its lexical class scope (constants, `def` target, class variables).  A
  cref whose `prev` is null is the top level.
* `refine C do ... end` makes a Refinement module (`ClassData.refined`, `attached` = C) that is
  in no ancestry; the module remembers its refinements in the `__refinements` ivar.
* `using M` replaces the *caller frame's* cref with a copy whose `refinements` Array holds the
  activated refinements (code defined earlier keeps the old cref, so it stays unrefined) and
  bumps `method_serial`.  From the first `using` on (`REFINEMENTS_ON`), a call-cache miss
  resolves through `lookup_method_refined(frame cref, klass, mid)`; a call site belongs to one
  cref, so its inline cache stays valid.  `super` from a refined method continues at C.
* Hidden compiler variables: `__fwd_rest/__fwd_kw/__fwd_blk` (anonymous `*`, `**`, `&`, `...`),
  `__it`, `%ff<offset>` (flip-flop state, kept in the enclosing method/program scope).
## Library layout (boot cost vs. coverage)
* Eager natives: everything `builtins::boot()` calls.
* Lazy natives: a constant registered with `prelude::lazy_const(owner, "Name", "saruby/x")`
  sits in the autoload table; first reference requires the internal feature, whose
  `prelude::load_feature` arm runs the Rust `init` (ENV, RbConfig, FileTest, File::Stat,
  Errno::*, Mutex/Queue/Monitor).  `require "strscan"` / `"json"` / `"zlib"` build their classes the same way.
  zlib (builtins/zlib.rs) is ext/zlib/zlib.c over Rust ports of zlib 1.3 (zlib_check.rs, zlib_deflate.rs,
  zlib_inflate.rs): the compressed bytes equal CRuby's; a running stream is taken out of its object, so a
  block or io callback that re-enters the object gets Zlib::InProgressError.
  json: `require "json"` loads the newest installed json gem's Ruby code (as rubygems activates it)
  over `json/ext/parser` and `json/ext/generator` done natively (json_parser.rs, json_generator.rs,
  json_fpconv.rs = the gem's Grisu2): JSON::VERSION picks the 3.x or the 2.x rules (both checked against
  json 3.1.0 and 2.13.2). builtins/json.rs is only the fallback when there is no reference ruby.
* Lazy Ruby: `prelude/lib/*.rb` are embedded and compiled on first `require`
  (`etc`, `timeout`, `monitor`, `time`; ARGF is autoloaded from `saruby/argf`).
* Everything else pure-Ruby comes from the reference ruby's stdlib: `build.rs` records
  `rubylibdir` and the gem dir (override at run time with `SARUBY_STDLIB=libdir:gemdir`);
  `$LOAD_PATH` is that ruby's default load path (site, vendor, rubylibdir, archdir); `require` searches
  the entries other than rubylibdir / archdir, then saruby's own features, then the stdlib and the
  bundled / newest gems (`prelude::stdlib_file`, looked up once per name).  The expanded load path is
  kept until `$LOAD_PATH` (or the cwd, for relative entries) changes.  `RbConfig` is that ruby's
  rbconfig.rb with saruby's binary, libdir and libsaruby.so in CONFIG.  Libraries whose C half is
  missing (`prelude::EXT_BACKED`: openssl, ripper, fiddle, nkf, ...) stay no-op requires.

## IO, processes, threads
* `RIo` (builtins/io.rs) is a raw fd plus read/write buffers; fd 1 writes go through the
  global stdout buffer (`kernel::out_write`).  `flush_all` runs before fork/exec and at exit.
  Encoding conversion is io.c's: a stream with an internal encoding or a newline decorator reads
  through a readconv into a char buffer and the char readers split that; writes go through
  rb_str_encode and a writeconv (builtins/io_conv.rs on econv.rs; docs/io_design.md "Encoding
  conversion").
* Process (builtins/process.rs) forks with libc directly; `Signal.trap` handlers set a pending
  bit and run at blocking points (`sleep`, `Process.kill` to self, waits, `IO.select`).
* Fibers (fiber.rs, CRuby's resume / yield / transfer / raise / kill rules): a fiber gets a
  16 MiB machine stack (mmap, guard page), a `Ctx` and a value stack at its first resume, and
  gives them back when it ends (right after the switch away from it) or is collected.  The
  machine side (stacks, the register switch, which also passes the value) is `fiber_os.rs`, the
  only unsafe code of fibers but for the one call of the switch.  Value stacks of FRAME slots or
  more are private views of a file of nils (`fiber_os::nil_slots`): a stack costs the pages it
  touches, and a released one gives back all but its first 64 KiB.  Fibers that are started
  trigger a collection once as many have started as held stacks after the last one, so dropped
  suspended fibers come back without waiting for the object heap to fill.
* Threads (builtins/thread.rs) are green: a Thread is a fiber chain entered from the main
  thread's scheduler (`fiber::thread_switch_in/out`).  A thread runs until it blocks in
  `thread::wait` on a blop (builtins/blop.rs: an op owned by a slab, named by an Id — Poll /
  Read / Write / Timer / Wake), or calls `Thread.pass`; there is no preemption.  The scheduler
  switches in only runnable threads and sleeps in the engine's pump (poll or epoll).  See
  [io_design.md](io_design.md).  Thread#kill unwinds with an internal exception that `rescue` skips
  and `ensure` does not put in `$!` (`srb_rescuable`, node_ensure).  A deadlock (the main thread's
  scheduler finds nothing runnable) raises `fatal` with CRuby's thread dump; a joiner is found by the
  Wake key of its parked op (the joined Thread's address).  ConditionVariable#wait queues a waiter
  and sleeps as Mutex#sleep does (a signal or Thread#wakeup ends it; the re-lock defers interrupts).

## Testing
Differential tests against CRuby (oracle): `../../../astro/sample/rubyharness`.
```
H=../../../astro/sample/rubyharness
ruby $H/tools/run_specs.rb --jobs 8 --timeout 15 --pattern 'string*.rb' --interp ./saruby --diff ruby --dir $H/t/hand
```
(`t/hand`, `t/method`, `t/syntax`, `t/spec`).  Single file: `diff <(./saruby f.rb 2>&1) <(ruby f.rb 2>&1)`.

## AOT
`./saruby --aot-compile x.rb` runs and then specializes every executed body into
`code_store/all.so`; later runs load it and swap dispatchers by Merkle hash (`--plain`
disables).  `SARUBY_AOT_VERBOSE=1` shows the rustc command.

* **Bodies are contiguous.**  `finish_iseq` copies each finished body into one block with
  `layout_tree` (parents before children), so a node sits at an offset from the body's root that
  only the tree's shape decides.  Baked code addresses a node, and so its inline cache, as
  `root + constant` (generator hook `contiguous?`); it never loads a child pointer.  Only laid-out
  trees are specialized or given baked code (`NF_LAID`).  The original nodes are left alone
  (about 1 MB and 0.6M instructions at boot).
* **Items are named by what is baked into them.**  `T_<x>` is the node hash merged with its children's
  item names and its baked operands (`@bake` values, speculated callees), so two sites with one hash
  that saw different things get their own code; only `SD_<hash>` (the dispatcher swap) is by hash alone.
* **Baked facts are hints.**  A `@bake` operand records what the interpreter saw at a site: the
  ivar slot (`slot`, node_ivar / node_iasgn / node_ivar_arith), the operand kinds of an operator
  besides fixnums (`ty`, from the site's `TyFb`), the callee kind of a call.  Baked items are named
  by the tree's hash and are not rewritten when another program reuses them, so the code tests
  each fact at run time (the slot against the site's cache) or keeps a complete out-of-line path
  for the other cases (`srb_arith_out`, `srb_aref_out`, `srb_ivar_get_out`): a stale or absent
  fact costs time, never correctness.
* **Inline builtins** (`src/inline_builtin.rs`).  A `node_call` whose cache held one of a table of builtins
  (Integer `even?` / `-@` / `succ` ..., Float `to_i`, Array `size` / `first` / `push`, Hash `size`, String
  `length` ...) is baked with that body's number (`bi`).  It evaluates the receiver and argument into
  registers; when `CallCache.bi` (set by `fill_cc`: the body of the cached entry, if it is the entry
  boot defined for that name, and whether the cached class is Integer / Float) matches and the serial
  is current, the body runs inline, with no staging, line store or cfunc frame.  Otherwise, or when the
  body declines (Bignum, frozen or full Array, ...), the site stages and calls out of line.  Bodies must
  not raise, call Ruby or look at a frame.  Builtins are recognised by method entry, never by function
  pointer (LLVM merges identical functions, so one builtin can have two addresses).  `srb_aref_out`
  answers Integer#[] the same way.
* An Array index site compares the receiver's class with `FAST_ARY_AREF` / `FAST_ARY_ASET` (rt.rs):
  the Array class while `[]` / `[]=` are the builtins, 0 after a redefinition.
