# koruby (ASTrust) — design notes

koruby is a Ruby implementation in Rust built on ASTrust (`../../lib/astrustgen.rb`,
`../../runtime/astrust.rs`), the Rust port of ASTro.  It is a rewrite of
`astro/sample/koruby_precise` (C) — same goals (CRuby drop-in, AOT by partial evaluation),
different implementation language.

## Pipeline

```
source --prism (C, vendor/prism) --> serialized AST --src/prism.rs + prism_ast.rs--> Rust AST
       --src/compile.rs--> ASTrust nodes (node.def) --interpreter: DISPATCH_* (node_dispatch.rs)
                                                    \-AOT: SPECIALIZE_* emit Rust --rustc--> code_store/all.so
```

* `node.def` — node kinds with Rust bodies.  `ruby koruby_gen.rb` (via `make`) generates
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
  (`make FEATURES=safe_stack`: every access checked, `#![forbid(unsafe_code)]`).  Code outside
  them uses only their common API.  See docs/safe_stack.md.

## Values (src/value.rs)
Fixnum (`...1`), Flonum (`..10`, CRuby encoding), heap pointer (`.000`), nil=0x04, false=0x0c,
true=0x14, undef=0x1c, Symbol `(id<<8)|0x34`.  No value is 0 → `RV` fits in one register.

## Objects (src/object.rs) and GC (src/gc.rs)
One fixed-size slot per object (`RObj { klass, flags, gc, body: Body }`); variable data in Rust
Vecs/Boxes owned by the slot.  Non-moving mark & sweep, as a backend of the ASTrust GC framework
(`../../runtime/gc`, `../../docs/gc.md`); `src/gc.rs` is koruby's binding.  `make GC=bitmap`
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

**Rooting (important):** heap objects are traced precisely; the machine stack and registers
are scanned conservatively, and the VM value stack too.  So Values in Rust locals are safe.
**Never keep a Value only inside a Rust heap buffer (Vec<Value>, Box) across an allocation or
a Ruby call** — if needed, put it in a Ruby Array (`ary_new_from`) held in a local.  Don't hold
`&mut` borrows into an object (e.g. `rary(v)`) across calls that can run Ruby code; iterate by
index and re-fetch (`ary_get(v, i)` with `ary_len(v)` checked each iteration).

## Methods
* Builtins are `CFunc = fn(&mut Ctx, self: Value, args: &[Value], blk: *const Block) -> RV`,
  registered with `define_cfunc(klass, "name", f, arity)` (arity -1 = variadic; checked).
  `args` comes from `with_args`: the staged slots themselves, or a copy of them under `safe_stack`.
  Keyword args to a cfunc arrive as a trailing Hash; `kw_arg(args)` returns it if the caller
  passed keywords.
* Call Ruby from Rust: `funcall(c, recv, sym, &args, blk)`, `yield1(c, blk, v)`,
  `yield_block(c, blk, &args)`, `call_proc(c, proc, &args, blk)`.
* Raise: `return err(c, vm().c_argument_error, "msg")`, `arg_err`, `type_err`, or
  `Err(raise_exc(c, exc_obj))`.
* Ruby-level built-ins live in `prelude/core.rb` (loaded at boot).

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
* Lazy natives: a constant registered with `prelude::lazy_const(owner, "Name", "koruby/x")`
  sits in the autoload table; first reference requires the internal feature, whose
  `prelude::load_feature` arm runs the Rust `init` (ENV, RbConfig, FileTest, File::Stat,
  Errno::*, Mutex/Queue/Monitor).  `require "strscan"` / `"json"` build their classes the same way.
* Lazy Ruby: `prelude/lib/*.rb` are embedded and compiled on first `require`
  (`etc`, `timeout`, `monitor`, `time`; ARGF is autoloaded from `koruby/argf`).
* Everything else pure-Ruby comes from the reference ruby's stdlib: `build.rs` records
  `rubylibdir` and the gem dir (override at run time with `KORUBY_STDLIB=libdir:gemdir`);
  `require` falls back to it after `$LOAD_PATH`.  Libraries whose C half is missing
  (`prelude::EXT_BACKED`: socket, date, zlib, openssl, ...) stay no-op requires.

## IO, processes, threads
* `RIo` (builtins/io.rs) is a raw fd plus read/write buffers; fd 1 writes go through the
  global stdout buffer (`kernel::out_write`).  `flush_all` runs before fork/exec and at exit.
* Process (builtins/process.rs) forks with libc directly; `Signal.trap` handlers set a pending
  bit and run at blocking points (`sleep`, `Process.kill` to self, waits, `IO.select`).
* Threads (builtins/thread.rs) are green: a Thread is a fiber chain entered from the main
  thread's scheduler (`fiber::thread_switch_in/out`).  A thread runs until it blocks in
  `thread::block_on` (join, Queue, Mutex, ConditionVariable, sleep, pipe read, `Thread.pass`);
  there is no preemption.

## Testing
Differential tests against CRuby (oracle): `../../../astro/sample/rubyharness`.
```
H=../../../astro/sample/rubyharness
ruby $H/tools/run_specs.rb --jobs 8 --timeout 15 --pattern 'string*.rb' --interp ./koruby --diff ruby --dir $H/t/hand
```
(`t/hand`, `t/method`, `t/syntax`, `t/spec`).  Single file: `diff <(./koruby f.rb 2>&1) <(ruby f.rb 2>&1)`.

## AOT
`./koruby --aot-compile x.rb` runs and then specializes every executed body into
`code_store/all.so`; later runs load it and swap dispatchers by Merkle hash (`--plain`
disables).  `KORUBY_AOT_VERBOSE=1` shows the rustc command.
