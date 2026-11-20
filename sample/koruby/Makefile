# koruby (ASTrust): node.def -> src/node_*.rs (astrustgen) -> cargo (libkoruby.so + koruby)
GEN = src/node_head.rs src/node_eval.rs src/node_dispatch.rs src/node_hash.rs src/node_specialize.rs src/node_dump.rs src/node_replace.rs
PROFILE ?= release
SHELL = /bin/bash

all: koruby

$(GEN): node.def koruby_gen.rb ../../lib/astrustgen.rb
	ruby koruby_gen.rb --input=node.def --output-dir=src

src/prism_ast.rs: tools/gen_prism_ast.rb vendor/prism/config.yml
	ruby tools/gen_prism_ast.rb

koruby: $(GEN) src/prism_ast.rs $(shell find src ../astrogre/src -name '*.rs') ../astrogre/node.def prelude/core.rb ../../runtime/astrust.rs
	cargo build --$(PROFILE)
	cp target/$(PROFILE)/koruby target/$(PROFILE)/libkoruby.so .

INTERP ?= ./koruby
ASTRO ?= $(HOME)/ruby/astro
include $(ASTRO)/sample/rubyharness/harness.mk
test bench: koruby

.PHONY: all
