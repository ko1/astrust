GEN = src/node_head.rs src/node_eval.rs src/node_dispatch.rs src/node_hash.rs src/node_specialize.rs src/node_dump.rs src/node_replace.rs

all: calc

$(GEN): node.def calc_gen.rb ../../lib/astrustgen.rb
	ruby calc_gen.rb --input=node.def --output-dir=src

calc: $(GEN) $(wildcard src/*.rs) ../../runtime/astrust.rs
	cargo build --release
	cp target/release/calc target/release/libcalc.so .

clean:
	cargo clean; rm -f calc libcalc.so $(GEN); rm -rf code_store
