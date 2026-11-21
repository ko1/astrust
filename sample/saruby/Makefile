# saruby (ASTrust): node.def -> src/node_*.rs (astrustgen) -> cargo (libsaruby.so + saruby)
GEN = src/node_head.rs src/node_eval.rs src/node_dispatch.rs src/node_hash.rs src/node_specialize.rs src/node_dump.rs src/node_replace.rs
PROFILE ?= release
# GC backend (../../runtime/gc, ../../docs/gc.md): marksweep | bitmap
GC ?= marksweep
SHELL = /bin/bash

all: saruby

$(GEN): node.def saruby_gen.rb ../../lib/astrustgen.rb
	ruby saruby_gen.rb --input=node.def --output-dir=src

src/prism_ast.rs: tools/gen_prism_ast.rb vendor/prism/config.yml
	ruby tools/gen_prism_ast.rb

# cargo features, e.g. FEATURES=raw_stack; a change of FEATURES rebuilds
FEATURES ?=
.features: FORCE
	@echo '$(FEATURES)' | cmp -s - $@ || echo '$(FEATURES)' > $@
FORCE:

saruby: $(GEN) src/prism_ast.rs build.rs tools/prism_serialize.c $(shell find src ../astrogre/src -name '*.rs') ../astrogre/node.def prelude/core.rb $(wildcard prelude/lib/*.rb) ../../runtime/astrust.rs $(wildcard ../../runtime/gc/*.rs) target/.gc-$(GC) .features
	cargo build --$(PROFILE) --features "gc-$(GC) $(FEATURES)"
	cp target/$(PROFILE)/saruby target/$(PROFILE)/libsaruby.so .

# remade when GC= changes, so the binary is rebuilt with the other backend
target/.gc-$(GC):
	mkdir -p target && rm -f target/.gc-* && touch $@

INTERP ?= ./saruby
ASTRO ?= $(HOME)/ruby/astro
include $(ASTRO)/sample/rubyharness/harness.mk
test bench: saruby

.PHONY: all FORCE
