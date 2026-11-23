# saruby Ractor — CRuby 4.0's Ractor on green threads

`src/builtins/ractor.rs` (`#![forbid(unsafe_code)]`) is ractor.rb / ractor.c / ractor_sync.c of CRuby 4.0
(behaviour checked against the 4.0.2 binary). Measurements, probes and tests:
`~/ruby/src/trials/2026-11-23-saruby-ractor/`.

## Model

Every Ractor's main thread is a green thread of thread.rs (docs/io_design.md): all Ractors share the one
interpreter thread, so **nothing runs in parallel**. What CRuby makes observable is kept:

| CRuby 4.0 | saruby |
|---|---|
| `rb_ractor_t` (id, name, loc, threads, ports, monitors, value, local storage) | `RRactor` (RData); a thread knows its Ractor (`RThread.ractor`, nil = main) |
| Ractor.new: `rb_proc_isolate_bang` (outer variables / `yield` → ArgumentError), self = the Ractor, arguments sent to the default port | the same; outer variables come from the compiler (`Iseq::outer_vars`, compile.rs `outer_rec`: every block strictly inside the owner of a variable it reads or writes; `yield` marks every enclosing block; a read whose value is unused does not count, as CRuby's compiler drops it; listed by prism constant id ≈ CRuby's ID order). Collected only for sources whose constant pool names `Ractor` (see cost below) |
| new Ractor runs at once on another core | it gets a first turn at once (`Thread.pass`), so `inspect` shows `blocking` / `running`, not `created` |
| Port: owner + id, queue per port, `receive` only by the owner, `close` only by the owner, queued messages stay receivable after close | the same (`PortQ`, a VecDeque traced by the GC) |
| Ractor#join / #value through a monitor port; `value` only for the successor; exception → RemoteError (cause, `#ractor`) | waits on the Ractor's end directly; join / value / select still use up a port id, so ids in `inspect` match |
| Ractor.select: first ready port or Ractor in argument order | the same |
| waits are not deadlock-detected (`Ractor.receive` with nobody to send hangs) | blop `Wake` waits with `deadlockable = false`; one key (main Ractor's address \| 2) notified by every send / close / end |
| Ractor-local storage, `store_if_absent` | a Hash per Ractor |

## Copy and move (obj_traverse_replace_i)

A message (and every Ractor.new argument) is passed by reference when shareable, else deep-copied, or moved
with `move: true`. One traversal (`Replace`, an explicit stack, no recursion) keeps identity and cycles (a
record table, its new objects rooted in an Array since the table is not traced).

- **copy**: each non-shareable object is `rb_obj_clone`d (`object::obj_dup(.., clone)`: singleton class,
  `initialize_copy`, frozen state), then its references in the clone (ivars incl. hidden ones and generic
  ivars, Array elements, Hash keys / values / default / default proc, Range ends) are replaced. Proc / Method:
  `TypeError: allocator undefined for Proc`; built-in data whose references are not all shareable:
  `Ractor::Error: can not copy X object.`; a Set (T_DATA in CRuby) only if its elements are shareable.
- **move**: a new object of the same class gets the old body (`mem::replace` of `Body`, flags, generic ivars);
  the old object becomes a frozen `Ractor::MovedObject` with an empty object body, so every method call
  (call sites cache on the class; type fast paths test the body) ends in `method_missing` / the overridden
  BasicObject methods: `Ractor::MovedError: can not send any methods to a moved object`. Built-in data, Proc,
  Method and Set: `can not move X object.`.

## Shareable

`shareable(v)`: immediates, classes / modules, Ractor, Port, ENV, and frozen objects of a kind CRuby allows
(`allow_frozen_shareable_p`: not Proc / Method / most built-in data; Time, Regexp, MatchData, Rational,
Complex, Encoding, IO are) whose references are shareable. Float, Bignum, Range, Rational, Complex and
Encoding count as frozen. Plain objects here that are T_DATA in CRuby (Dir, ConditionVariable, WeakMap,
WeakKeyMap) are not. A yes is remembered in `FL_SHAREABLE` (bit 64; dup / clone make new objects, so it is
never copied). `make_shareable` freezes depth first by calling `#freeze` (user overrides run, as in CRuby),
isolates Procs (`Proc's self is not shareable`, outer variables' values must be shareable), accepts a
Method whose receiver is shareable, and raises `can not make shareable object for ...` for the rest.
`Set#freeze` freezes its table (CRuby's Set is frozen-shareable).

## Isolation checks, and why single-Ractor code does not pay

All checks are gated on `ractor::multi()` (`LIVE != 0`: a non-main Ractor is alive; one relaxed atomic load)
and sit on paths that are already slow:

| check (CRuby message) | where |
|---|---|
| constant with a non-shareable value read by a non-main Ractor | `rt::srb_const_get` (the inline cache's miss path), `srb_const_path`, `Module#const_get`. While `multi()`, only shareable values are cached, and the first Ractor.new bumps `const_serial`, so a cache hit never needs a check. Lexical hits say "by non-main ractor.", others "by non-main Ractor." (`Object::C`) as CRuby |
| constant assignment of a non-shareable value | `srb_cdecl`, `Module#const_set` |
| global variables except the Ractor-local ones (`$~ $_ $! $@ $1.. $stdout $stderr $stdin $> $/ $VERBOSE $DEBUG $$ $? ...`) | `srb_gvar_get / set` (node_gvar is always a call), `global_variables` |
| class variables | `srb_cvar_get / set`, `class_variable_get / set` |
| class / module ivars: non-shareable read, any write | writes: `srb_ivar_set`; reads: attr reader on a singleton class (`ractor::attr_get`, out of line from `invoke_untraced`; the attr fast path is for plain objects only) and `instance_variable_get` raise at once. `@x` itself (node_ivar's miss path: a class's ivars never hit the object cache) is `&Ctx -> Value` and cannot raise: it records the read and sets the interrupt bit, so the error is raised at the Ractor's next check point (the next call site, loop back edge, or the end of its block), before the value reaches a method call. Making that path return `RV` cost poly 1.4-2 % (baked code) |

## Cost (trial `2026-11-23-saruby-ractor`, instruction counts, CODEGEN_UNITS=1)

The 15 icount benchmarks, interp and AOT, startup and optcarrot are in the trial's README. What did not
fit the 0.3 % budget and was changed: an `RV` from node_ivar's miss path (poly AOT +2.1 %); outer variables
collected in `resolve()` for every block (startup +1.8 %); any arm added to `analyze_rec` (startup +0.5 %).

## Differences from CRuby 4.0.2 (known)

- No parallelism; a Ractor is preempted only at the green threads' check points.
- `$stdout` / `$stderr` / `$VERBOSE` are not per-Ractor objects; `require` in a non-main Ractor runs there.
- A class ivar read with `@x` by a non-main Ractor raises at the next check point, not at the read
  (the value is held until then; no method can be called with it first). Its backtrace starts there.
- The outer-variable check needs the source of the block to name `Ractor`: `Ractor.new(&pr)` with a `pr`
  made in a file that never mentions Ractor is not rejected, and `make_shareable` of such a proc checks
  only its self.
- `p moved_obj` raises MovedError (CRuby: NoMethodError, its C-level funcall skips the boot-time
  `method_missing`).
- Copying a Mutex raises (`allocator undefined`; CRuby makes a new Mutex); `Ractor::Port#dup` says
  "allocator undefined" (CRuby: "[bug] frozen object (Ractor::Port) allocated").
- A Ractor's other threads are not killed when its main thread ends; a moved object's `object_id` stays with
  the old object.
- Range literals with String ends are not frozen deep (CRuby compiles them as frozen literals), so
  `Ractor.shareable?("a".."b")` is false here; freezing an object does not freeze its singleton class.
