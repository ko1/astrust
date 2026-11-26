# saruby (ASTrust): node.def -> src/node_*.rs (astrustgen) -> cargo (libsaruby.so + saruby)
GEN = src/node_head.rs src/node_eval.rs src/node_dispatch.rs src/node_hash.rs src/node_specialize.rs src/node_dump.rs src/node_replace.rs
PROFILE ?= release
# GC backend (../../runtime/gc, ../../docs/gc.md): marksweep | bitmap
GC ?= marksweep
SHELL = /bin/bash

all: $(if $(COVERAGE),cov/saruby,saruby)

$(GEN): node.def saruby_gen.rb ../../lib/astrustgen.rb
	ruby saruby_gen.rb --input=node.def --output-dir=src

src/prism_ast.rs: tools/gen_prism_ast.rb vendor/prism/config.yml
	ruby tools/gen_prism_ast.rb

# the CRuby saruby follows (src/lib.rs RUBY_VERSION): its stdlib and gems are what require loads
REF_RUBY ?= $(firstword $(wildcard $(HOME)/.rbenv/versions/4.0.10/bin/ruby) ruby)
export SARUBY_REF_RUBY := $(REF_RUBY)

# cargo features, e.g. FEATURES=raw_stack; a change of FEATURES rebuilds
FEATURES ?=
.features: FORCE
	@echo '$(FEATURES)' | cmp -s - $@ || echo '$(FEATURES)' > $@
FORCE:

saruby: $(GEN) src/prism_ast.rs build.rs tools/prism_serialize.c $(shell find src ../astrogre/src -name '*.rs') ../astrogre/node.def prelude/core.rb $(wildcard prelude/lib/*.rb prelude/lib/*/*.rb prelude/lib/*/*/*.rb prelude/lib/*/*/*/*.rb) $(wildcard ../../runtime/*.rs) $(wildcard ../../runtime/gc/*.rs) target/.gc-$(GC) .features
	cargo build --$(PROFILE) --features "gc-$(GC) $(FEATURES)"
	cp target/$(PROFILE)/saruby target/$(PROFILE)/libsaruby.so .

# make COVERAGE=1: source-coverage build (tools/coverage.sh).  Own target dir and output dir, so
# ./saruby and target/ are never touched.  RUSTFLAGS replaces .cargo/config.toml's, so repeat it.
cov/saruby: $(GEN) src/prism_ast.rs build.rs tools/prism_serialize.c $(shell find src ../astrogre/src -name '*.rs') ../astrogre/node.def prelude/core.rb $(wildcard prelude/lib/*.rb prelude/lib/*/*.rb prelude/lib/*/*/*.rb prelude/lib/*/*/*/*.rb) $(wildcard ../../runtime/*.rs) $(wildcard ../../runtime/gc/*.rs) .features
	CARGO_TARGET_DIR=target-cov RUSTFLAGS="-C prefer-dynamic -C target-cpu=native -C instrument-coverage" cargo build --$(PROFILE) --features "gc-$(GC) $(FEATURES)"
	mkdir -p cov && cp target-cov/$(PROFILE)/saruby target-cov/$(PROFILE)/libsaruby.so cov/

# remade when GC= changes, so the binary is rebuilt with the other backend
target/.gc-$(GC):
	mkdir -p target && rm -f target/.gc-* && touch $@

INTERP ?= ./saruby
# the oracle for make test / bench: installed CRuby master, the same binary koruby uses (user 2026-11-26)
RUBY ?= $(firstword $(wildcard $(HOME)/ruby/install/master/bin/ruby) ruby)
# the shared Ruby test/bench harness (its own repository since 2026-11-25; was astro/sample/rubyharness)
RUBYHARNESS ?= $(HOME)/ruby/rubyharness
# optional: build-only hosts (sp3 / sp4) have no harness checkout
-include $(RUBYHARNESS)/harness.mk
test bench: saruby

.PHONY: all FORCE
