# ASTrust: ASTro in Rust

ASTrust is the Rust counterpart of [ASTro](../astro/) (AST-based Reusable Optimization
Framework).  You write an AST interpreter's node semantics once in `node.def` (in Rust); the
generator derives an interpreter, a Merkle-hash function per node kind, and a **partial
evaluator** that emits specialized Rust code for a given AST.  rustc compiles that code into a
shared object that is `dlopen`ed and swapped in for the interpreter's dispatchers.

```
lib/astrustgen.rb     the generator (node.def -> node_{head,eval,dispatch,hash,specialize,dump,replace}.rs)
runtime/astrust.rs    language-independent runtime: hashing, SpecCtx, CodeStore (rustc + dlopen), TieredStore
runtime/gc/           GC framework: language binding + pluggable backends (docs/gc.md)
sample/calc/          smallest example (integer calculator with loops)
sample/koruby/        a Ruby implementation (rewrite of astro/sample/koruby_precise in Rust)
```

## How it differs from ASTro (C)

| | ASTro (C) | ASTrust (Rust) |
|---|---|---|
| node bodies | C | Rust |
| lazy child | `(NODE *, dispatcher fnptr)`; the C compiler must constant-fold the pointer | `EVAL_x<D: Disp>`: generic over the child's dispatcher **type**; the specializer instantiates it with the child's zero-sized SD type, so static dispatch is guaranteed by monomorphization |
| child list (`NODES`) | `NODE **argv@children` | `D: DispList` (`len`, `nth`); SD list types unroll `nth` |
| specialized code | C → gcc → `.so` | Rust → rustc (`cdylib`, `--extern` the interpreter's own `dylib` crate, `-C prefer-dynamic`) → `.so` |
| error propagation | `RESULT` struct | `Result<Value, Esc>` with a zero-sized `Esc` (payload in the context): one register |

The interpreter itself is a Rust `dylib`, so `#[inline]` evaluators (`EVAL_*`) are inlined into
the separately compiled SD crate, and both share one copy of the interpreter's state.

## node.def

```
NODE_DEF [@noinline] [@always_inline] [@canonical=BASE]
node_add(c: &mut Ctx, n: NodePtr, <common params...>, l: NODE, r: NODE) -> RESULT
{
    EVAL_ARG!(c, l).wrapping_add(EVAL_ARG!(c, r))
}
```
Operands: `x: NODE` (lazy child, `EVAL_ARG!(c, x)`), `x@child: T` (evaluated by the glue first),
`xs: NODES` (`EVAL_LEN!(xs)`, `EVAL_NTH!(c, xs, i)`), `x@ref: T` (stored inline, body gets
`&mut T`; caches), scalars `u32 i32 u64 i64 usize bool f64` (baked into SD code as literals),
anything else is opaque (read from the node at run time; the embedder's generator subclass
decides hashing).  Embedders subclass `ASTrustGen::NodeDef::Node` for the common-parameter
count, result type, how to unwrap a child's result (`e?` for `Result`), and opaque operands —
see `sample/calc/calc_gen.rb` (minimal) and `sample/koruby/koruby_gen.rb`.

### @child storage

Where an `@child` operand's value lives between its evaluation and the node body is a generator
hook, the counterpart of ASTroGen's `child_storage_decl` / `child_storage_expr`:

```ruby
class Node < ASTrustGen::NodeDef::Node
  def child_storage_stmt(op, value) = "let v_#{op.name} = #{value};"   # default: a Rust local
  def child_storage_expr(op) = "v_#{op.name}"                          # the argument given to EVAL
end
```

`child_storage_stmt` is emitted once per `@child`, in operand order, in both the interpreter
dispatcher and the specialized code; `child_index(op)` is the operand's position among the
node's `@child`s.  The default suits an embedder with no GC or one that scans the machine stack.
A precise-GC embedder (docs/gc.md) stages the value in a root-scanned slot instead, e.g.
`"*fp.add(#{base + child_index(op)}) = #{value};"` and `"*fp.add(#{base + child_index(op)})"`.

## GC

`runtime/gc/` is a collector framework in the manner of ASTro's `runtime/precise_gc`: the
language implements `GcLang` (roots, edges as mutable slots, finalization), a backend
implements `GcBackend`, and `Gc<L, B>` joins them at compile time.  Two non-moving backends
exist (`marksweep`, `bitmap`); koruby selects one with `make GC=<name>`.  See `docs/gc.md`.

### Common parameters

The common parameters are copied through as written, so they can be any Rust types.  A frame
of local variables does not have to be a raw pointer: koruby passes `fp: Fp`, which in its
checked configuration is `&'static [Cell<Value>; N]`.  A slot index is a scalar operand and
therefore a literal in specialized code, so `fp[i]` there is checked at compile time and costs
nothing at run time (`sample/koruby/docs/safe_stack.md`).  No generator hook was needed.

## Code store

`CodeStore` keeps one file per emitted item (`items/T_<hash>.rs`, `L_<hash>.rs`, `SD_<hash>.rs`,
content-addressed so bakes accumulate) and links them into `all.so`; `lookup(hash)` returns
`SD_<hash>`.  Each `.so` embeds the interpreter's build id and is ignored when it does not match.
`TieredStore` adds a shared preload store (the embedder's prelude, baked once per build) in
front of the per-program store.

## Quick start

```sh
cd sample/calc && make
./calc --plain 's=0; i=0; 100000000 times { i = i + 1; s = s + i % 7 * 3 }; s'   # interpreter
./calc --aot-compile '...'   # run, then specialize + rustc into code_store/all.so
./calc '...'                 # uses the specialized code
```
