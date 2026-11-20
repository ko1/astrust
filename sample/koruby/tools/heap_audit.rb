#!/usr/bin/env ruby
# heap_audit.rb [--md]: what is left of the migration to the context-borrowing heap accessors
# (docs/safe_heap.md).  Per file: call sites of each old accessor, and `unsafe` occurrences.
# `fn->&'static mut`: functions of the file that return a `&'static mut` (its own unsound accessors).
# `static mut`: mutable statics declared in the file.
# Exit 1 if a converted file (one that imports heap::no_legacy) still names an old accessor.
# Generated sources (src/node_*.rs, src/prism_ast.rs) are not counted; node.def is.
ROOT = File.expand_path('..', __dir__)
BODY = %w[obj rary rstr rhash rclass rproc rmethod rbig rdata ivars_of str_bytes]
ARY  = %w[ary_new ary_new_capa ary_new_from ary_from_vec ary_push ary_len ary_get ary_to_vec]
ALLOC = %w[str_new str_from_vec str_from_vec_enc hash_new float_new]
COLS = { 'obj' => %w[obj], 'rary' => %w[rary], 'rstr' => %w[rstr str_bytes], 'rhash' => %w[rhash], 'rclass' => %w[rclass],
         'rproc' => %w[rproc rmethod], 'rbig' => %w[rbig], 'rdata' => %w[rdata], 'ivars_of' => %w[ivars_of],
         'ary_*' => ARY, 'vm' => %w[vm] }
md = ARGV.delete('--md')
files = Dir[File.join(ROOT, 'src/**/*.rs')].reject { _1 =~ %r{/src/(node_\w+|prism_ast)\.rs\z} }.sort + [File.join(ROOT, 'node.def')]

def strip(src) = src.gsub(%r{//[^\n]*}, '')            # line comments (doc comments included)
def count(src, name) = src.scan(/(?<![\w.])#{name}(?:::<[^>]*>)?\(/).size

rows = []; bad = []
files.each do |f|
  rel = f.sub(ROOT + '/', '')
  raw = File.read(f); src = strip(raw)
  # definitions are not call sites
  body = src.gsub(/\bfn\s+\w+/, 'fn _')
  c = COLS.transform_values { |ns| ns.sum { count(body, _1) } }
  uns = src.scan(/\bunsafe\b/).size
  converted = src =~ /^\s*use crate::heap::no_legacy::\*/ ? true : false
  helpers = src.scan(/\bfn\s+\w+[^{;]*->[^{;]*&'static mut/).size   # local accessors handing out &'static mut
  if converted
    left = (BODY + ARY + ALLOC + %w[vm]).select { count(body, _1) > 0 || body =~ /::#{_1}\(/ }
    bad << "#{rel}: #{left.join(' ')}" unless left.empty?
  end
  statics = src.scan(/\bstatic\s+mut\s+\w+\s*:/).size
  rows << [rel, c, uns, converted, raw.include?('#![forbid(unsafe_code)]'), helpers, statics]
end

heads = COLS.keys
sep = md ? ' | ' : ' '
fmt = ->(a) { md ? "| #{a.join(' | ')} |" : a.join(sep) }
w = rows.map { _1[0].size }.max
puts fmt.([md ? 'file' : 'file'.ljust(w), *heads.map { _1.rjust(6) }, 'sum'.rjust(5), "fn->&'static mut", 'static mut', 'unsafe', 'state'])
puts fmt.(['---'] * (heads.size + 6)) if md
tot = Hash.new(0); tu = 0; th = 0; ts = 0
rows.each do |rel, c, uns, conv, forbid, helpers, statics|
  s = c.values.sum
  next if s == 0 && uns == 0 && !conv && helpers == 0 && statics == 0
  heads.each { tot[_1] += c[_1] }; tu += uns; th += helpers; ts += statics
  state = conv ? (forbid ? 'converted, forbid(unsafe_code)' : 'converted') : (forbid ? 'forbid(unsafe_code)' : '')
  puts fmt.([md ? rel : rel.ljust(w), *heads.map { c[_1].to_s.rjust(6) }, s.to_s.rjust(5), helpers.to_s.rjust(16), statics.to_s.rjust(10), uns.to_s.rjust(6), state])
end
puts fmt.([md ? 'total' : 'total'.ljust(w), *heads.map { tot[_1].to_s.rjust(6) }, tot.values.sum.to_s.rjust(5), th.to_s.rjust(16), ts.to_s.rjust(10), tu.to_s.rjust(6), ''])
unless bad.empty?
  warn "converted files still naming old accessors:\n  " + bad.join("\n  ")
  exit 1
end
