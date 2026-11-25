# saruby — Sort of A Ruby

A Ruby implementation built on **ASTrust**, the Rust version of the ASTro framework (an AST
interpreter made fast by partial evaluation). It is a rewrite in Rust of
[astro/sample/koruby_precise](https://github.com/ko1/astro), which is written in C. It has two goals:

- to be a **drop-in replacement for CRuby**, and
- to find out **how close safe Rust can get to the C implementation** (the "sa" could as well be SAfe). `unsafe` is to be confined
  to external interfaces and a small, auditable core (a "Trusted Ruby").

> Experimental. Much of Ruby does not work yet, and the internals change often.

## Usage

Requirements: a stable Rust toolchain, Ruby 3.4 or later (for the generator), CRuby 4.0.10 as the source of
the pure-Ruby standard library and gems (`make REF_RUBY=<ruby>`, default `~/.rbenv/versions/4.0.10/bin/ruby`,
else `ruby`), and a C compiler (the bundled prism parser is built from source).

saruby follows CRuby 4.0.10: `RUBY_VERSION`, `RUBY_RELEASE_DATE`, `RUBY_REVISION`, RbConfig's version
entries and `-v` say 4.0.10, but `RUBY_ENGINE` is `"saruby"`. Code that tests `RUBY_ENGINE == "ruby"` takes
its non-CRuby branch (gems choosing a C extension, mspec's `raise_consistent_error` / `fixnum_max`
helpers, `platform_is :ruby` guards).

```sh
make                              # node.def -> src/node_*.rs (astrustgen) -> cargo -> ./saruby
./saruby -e 'puts "hello"'
./saruby prog.rb                  # uses baked code if there is any, otherwise interprets
./saruby --plain prog.rb          # always interprets
./saruby --aot-compile prog.rb    # run, then specialize the AST and compile it with rustc (into code_store/)
./saruby prog.rb                  # later runs use the baked code
```

| build | meaning |
|---|---|
| `make` (default) | the value stack has no `unsafe` (`src/stack.rs`, `#![forbid(unsafe_code)]`) |
| `make FEATURES=raw_stack` | the value stack uses raw pointers (`src/stack_raw.rs`); for measuring what safety costs |
| `make GC=marksweep` (default) / `make GC=bitmap` | GC backend (`../../docs/gc.md`) |

| environment variable | meaning |
|---|---|
| `SARUBY_GC_STRESS=n` | collect every n-th allocation (finds missing roots) |
| `SARUBY_NO_BI=1` | bake without inline builtin bodies (`src/inline_builtin.rs`; for A/B) |
| `SARUBY_NO_IT=1` | bake without inline builtin iterators (`src/inline_iter.rs`; for A/B) |
| `SARUBY_SD_NAMES=1` | print which method or block each baked function `SD_<hash>` is (for reading profiles) |
| `SARUBY_AOT_VERBOSE=1` | show rustc's output while baking |
| `ASTRUST_SPEC_DEPTH=n` | how deep callees are compiled into their call sites (default 2; 0 turns it off) |

## How it works

- **Parser**: prism (C, bundled). The serialized AST is read on the Rust side and turned into a tree
  of `node.def` nodes (`src/compile.rs`).
- **Nodes**: about 100 kinds. Their bodies are Rust, written in `node.def`; the ASTrust generator
  produces the interpreter and the partial evaluator from them.
- **AOT**: `--aot-compile` specializes the AST into Rust code, guided by the inline caches the run
  has filled. A call site gets the (small) callee it saw compiled into it, and keeps only the path
  for the kind of callee it saw (method, attribute reader or writer, builtin). Nodes are not
  rewritten at run time.
- **Values**: tagged as in CRuby (fixnums, flonums and symbols are immediates). An object is a
  fixed-size slot; up to 3 instance variables and strings of up to 23 bytes live inside it.
- **GC**: non-moving mark-sweep. References in the heap and global roots are traced exactly; the
  machine stack and the value stack are scanned conservatively (making the roots precise is in
  progress).
- **Fiber / Thread**: fibers switch to their own stacks; threads are green threads.
- **Regexp**: `../astrogre`, also built on ASTrust, aiming at Onigmo compatibility.
- **Standard library**: the core is Rust (`src/builtins/`), some of it Ruby (the prelude).
  Pure-Ruby libraries (`set`, `optparse`, `fileutils`, ...) are loaded from the CRuby library
  directory recorded at build time.

Details are in `docs/design.md`; plans are in `docs/roadmap.md`.

## State of `unsafe`

The rule is: safe everywhere except external interfaces and an audited core. This is unfinished.

- **Value stack**: no `unsafe` in the default build. A frame is `&[Cell<Value>; 65536]`; in baked
  code slot indices are constants, so the bounds checks disappear (`docs/safe_stack.md`).
- **Access to object bodies**: being moved to accessors that borrow the context (`c.ary(v)`,
  `c.ary_mut(v)`). Holding a reference into a body across a call that takes the context is then a
  compile error, so the aliasing rule and the GC rule "no reference across an allocation" become
  one borrow-checker rule. Arrays are done: `src/array.rs` and `src/builtins/array_methods.rs` are
  `#![forbid(unsafe_code)]`. `tools/heap_audit.rb` counts what is left, and `docs/safe_heap.md` has
  the recipe for converting a module.
- **What remains** (about 570 occurrences outside generated code, as of 2026-11-21): accessors not
  yet converted, the GC core, the conservative scan of the machine stack, external interfaces
  (prism, libc, `dlopen`, fiber switching), and reads of node operands.
- **What types cannot guarantee**: that the object a `Value` names is alive (rooting). With an
  evaluator that recurses over the tree this cannot be proved by types alone; it is a discipline,
  checked with the GC stress mode.

## Compatibility and speed (as of 2026-11-21)

CRuby is the reference. Tests come from rubyharness (`~/ruby/rubyharness`, shared with ASTro's Ruby
samples; `make RUBYHARNESS=<dir>`), which compares output with CRuby's line by line.

- **t/hand** (about 1,700 files): 23,069 lines match, 10,419 do not.
- **optcarrot** (Lan_Master.nes, 180 frames): runs, and the checksum equals CRuby's.
- **Micro-benchmarks** (the 53 in rubyharness): every output equals CRuby's.

Speed is recorded per benchmark, as measured on one machine under the same conditions; no average
over a benchmark set is used.

| optcarrot (fps, range of 5 runs) | |
|---|---|
| CRuby 4.1.0dev | 53.7–54.7 |
| CRuby + YJIT | 294–299 |
| CRuby + ZJIT | 311–316 |
| koruby_precise AOT (ASTro, C) | 313–321 |
| saruby interpreter | 71–74 |
| saruby AOT (safe, the default) | 125–129 |
| saruby AOT (raw-pointer stack) | 130–131 |

On the 53 micro-benchmarks, saruby AOT (safe) is faster than koruby_precise AOT on 33, slower on
19, and within run-to-run variation on 1 (a benchmark counts as faster or slower only when the
min–max ranges of 5 runs do not overlap). The ones it loses are dominated by method and block
calls. The optcarrot gap has the same cause, made worse by the size of the baked code
(instruction-cache misses).

## Layout

```
node.def            node definitions (bodies in Rust)
saruby_gen.rb       saruby's settings for the ASTrust generator
src/compile.rs      prism AST -> node tree (placement of locals, analysis of variables captured by blocks)
src/vm.rs           classes, methods, calls, frames, inline caches
src/rt.rs           runtime functions called from node bodies (srb_*)
src/heap.rs         sound access to object bodies (the heap token)
src/stack.rs        the value stack (no unsafe) / stack_raw.rs (raw pointers)
src/gc.rs           the language side of the GC framework (../../runtime/gc)
src/builtins/       builtin classes
prelude/            builtins written in Ruby
docs/               design documents
tools/              generator scripts and measurement helpers
vendor/prism/       the bundled prism
```

## Name

saruby stands for "Sort of A Ruby". Runtime functions are prefixed `srb_`, environment variables
`SARUBY_`.
