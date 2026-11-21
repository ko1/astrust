# astrogre (ASTrust): node.def -> src/node_*.rs (astrogre_gen.rb) -> cargo (libastrogre.rlib + are)
GEN = src/node_head.rs src/node_eval.rs src/node_dispatch.rs src/node_hash.rs src/node_specialize.rs src/node_dump.rs src/node_replace.rs src/node_free.rs

all: are

$(GEN): node.def astrogre_gen.rb ../../lib/astrustgen.rb
	ruby astrogre_gen.rb --input=node.def --output-dir=src

are: $(GEN) $(wildcard src/*.rs) $(wildcard ../../runtime/*.rs)
	cargo build --release
	cp target/release/are target/release/libastrogre.rlib .

test: are
	cd test && ./run.sh

clean:
	cargo clean; rm -f are libastrogre.rlib; rm -rf code_store

.PHONY: all test clean
