# ASTrust: ASTro framework in Rust for trustworthy execution

> Experimental. Interfaces change without notice.

ASTrust is the Rust counterpart of [ASTro](https://github.com/ko1/astro) (AST-based Reusable
Optimization Framework). You write the semantics of an AST interpreter's nodes once, in Rust, in
a `node.def` file. From that one description the generator derives

- a tree-walking **interpreter**,
- a structural (Merkle) **hash** for every subtree, and
- a **partial evaluator** that, for a concrete AST, emits Rust code specialized to that tree.

rustc compiles the emitted code into a shared object, which is loaded and swapped in for the
interpreter's dispatchers. There is no JIT compiler to write and no second description of the
language: the specialized code is the interpreter's own node bodies, monomorphized over the tree.

The "trust" is deliberate. The goal is to see how close a **safe Rust** implementation can get to
the C one, with `unsafe` confined to external interfaces and a few small audited modules.

```
lib/astrustgen.rb     the generator: node.def -> node_{head,eval,dispatch,hash,specialize,dump,replace}.rs
runtime/astrust.rs    language-independent runtime: hashing, SpecCtx, CodeStore (rustc + dlopen), TieredStore
runtime/gc/           GC framework: a language binding and pluggable backends (docs/gc.md)
sample/calc/          the smallest example: an integer calculator with loops
sample/astrogre/      a Ruby-compatible regular expression engine (Onigmo semantics), with an `are` grep tool
sample/saruby/        saruby (Sort of A Ruby): a Ruby implementation, the main testbed
```

## Quick start

Requirements: a recent stable Rust toolchain, Ruby 3.4 or later (the generator is a Ruby script),
and a C compiler for saruby (it embeds the prism parser).

```sh
cd sample/calc && make
./calc --plain 's=0; i=0; 100000000 times { i = i + 1; s = s + i % 7 * 3 }; s'   # interpreter
./calc --aot-compile '...'   # run, then specialize + rustc into code_store/all.so
./calc '...'                 # uses the specialized code
```

```sh
cd sample/saruby && make
./saruby -e 'puts "hello"'
./saruby --aot-compile prog.rb && ./saruby prog.rb
```

## How it differs from ASTro (C)

| | ASTro (C) | ASTrust (Rust) |
|---|---|---|
| node bodies | C | Rust |
| lazy child | `(NODE *, dispatcher fnptr)`; the C compiler must constant-fold the pointer | `EVAL_x<D: Disp>`: generic over the child's dispatcher **type**. The specializer instantiates it with the child's zero-sized type, so static dispatch is guaranteed by monomorphization |
| child list (`NODES`) | `NODE **argv@children` | `D: DispList` (`len`, `nth`); specialized list types unroll `nth` |
| specialized code | C → gcc → `.so` | Rust → rustc (`cdylib`, `--extern` the interpreter's own `dylib` crate, `-C prefer-dynamic`) → `.so` |
| error propagation | `RESULT` struct | `Result<Value, Esc>` with a zero-sized `Esc` (the payload is in the context): one register |
| frame of locals | raw pointer into the value stack | any Rust type; saruby's default is a checked `&[Cell<Value>; N]` |

The interpreter itself is a Rust `dylib`, so `#[inline]` evaluators (`EVAL_*`) are inlined into
the separately compiled specialized crate, and both share one copy of the interpreter's state.

## node.def

```
NODE_DEF [@noinline] [@canonical=BASE]
node_add(c: &mut Ctx, n: NodePtr, <common params...>, l: NODE, r: NODE) -> RESULT
{
    EVAL_ARG!(c, l).wrapping_add(EVAL_ARG!(c, r))
}
```

| operand | meaning |
|---|---|
| `x: NODE` | lazy child; evaluate with `EVAL_ARG!(c, x)` |
| `x@child: T` | child evaluated by the glue before the body runs |
| `xs: NODES` | child list: `EVAL_LEN!(xs)`, `EVAL_NTH!(c, xs, i)` |
| `x@ref: T` | stored inline in the node, the body gets `&mut T` (inline caches) |
| `u32 i32 u64 i64 usize bool f64` | scalars, baked into specialized code as literals |
| anything else | opaque: read from the node at run time; the embedder decides how it is hashed |
| `x@spec: NODE` | a **speculated callee**, not stored in the node. At specialize time the embedder names a node (for example the method entry a call site's cache holds) to compile into this site; the body compares `SPEC_HASH!(x)` with the run-time target's hash and calls it statically with `EVAL_SPEC!(c, x, ptr, ...)` |
| `x@bake: u32` | a scalar computed at specialize time (0 in the interpreter), e.g. what kind of callee a call site has seen, so the body keeps only that case inline |

`NODE_TEST { ... }` after a node's body defines the node **evaluated as a condition**: it returns
the embedder's test type (a `bool` in a `Result`, say) directly, and `if` / `while` evaluate their
condition with `EVAL_TEST!(c, x)`. A comparison then branches without materializing a boolean
value. Nodes without a test body fall back to converting their value.

Embedders subclass `ASTrustGen::NodeDef::Node` to set the common-parameter count, the result type,
how a child's result is unwrapped (`e?` for a `Result`), how opaque operands are hashed and
specialized, and the hooks above. `sample/calc/calc_gen.rb` is the minimal case,
`sample/saruby/saruby_gen.rb` the full one.

### Common parameters

The parameters shared by every evaluator are copied through as written, so they can be any Rust
types. A frame of local variables does not have to be a raw pointer: saruby passes `fp: Fp`, which
is `&'static [Cell<Value>; N]` by default. A slot index is a scalar operand and therefore a
literal in specialized code, so `fp[i]` there is checked at compile time and costs nothing at run
time (`sample/saruby/docs/safe_stack.md`).

### @child storage

Where an `@child` operand's value lives between its evaluation and the node body is a generator
hook, the counterpart of ASTroGen's `child_storage_decl` / `child_storage_expr`:

```ruby
class Node < ASTrustGen::NodeDef::Node
  def child_storage_stmt(op, value) = "let v_#{op.name} = #{value};"   # default: a Rust local
  def child_storage_expr(op) = "v_#{op.name}"                          # the argument given to EVAL
end
```

The default suits an embedder with no GC or one that scans the machine stack. A precise-GC
embedder stages the value in a root-scanned slot instead (`docs/gc.md`).

## Code store

`CodeStore` keeps one file per emitted item (`items/T_<hash>.rs`, `L_<hash>.rs`, `SD_<hash>.rs`),
content-addressed so that bakes accumulate, and links them into `all.so`; `lookup(hash)` returns
`SD_<hash>`. Each `.so` embeds the interpreter's build id and is ignored when it does not match.
`TieredStore` puts a shared preload store (the embedder's prelude, baked once per build) in front
of the per-program store.

Specialization happens when code is baked, from what the run has observed (inline caches), not by
rewriting nodes at run time: the only thing patched in a running interpreter is a node's
dispatcher, when baked code for its hash is found.

## GC

`runtime/gc/` is a collector framework in the manner of ASTro's `runtime/precise_gc`. The language
implements `GcLang` (root enumeration, edges visited as mutable slots, finalization); a backend
implements `GcBackend` (allocation, collection, write-barrier hook, pinning); `Gc<L, B>` joins the
two at compile time. Two non-moving backends exist (`marksweep`, `bitmap`); saruby selects one with
`make GC=<name>`. What a precise or moving collector still needs from an embedder is listed in
`docs/gc.md`.

## Samples

- **calc** — about 100 lines of `node.def`; read this first.
- **astrogre** — regular expressions as continuation-passing match nodes; the search loop is itself
  a node, so baking fuses the loop and the pattern into one function.
- **saruby** — a Ruby implementation aiming at CRuby compatibility. See
  [`sample/saruby/README.md`](sample/saruby/README.md) for what runs, how fast, and how much
  `unsafe` is left.
