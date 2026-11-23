#!/usr/bin/env ruby
# heap_audit.rb [--md]: what is left of the migration to the context-borrowing heap accessors
# (docs/safe_heap.md).  Per file: call sites of each old accessor, and `unsafe` occurrences.
# `fn->&'static mut`: functions of the file that return a `&'static mut` (its own unsound accessors).
# `static mut`: mutable statics declared in the file.
# Exit 1 if a converted file (one that imports heap::no_legacy) still names an old accessor, or a file
# that imports heap::no_legacy_body names an object accessor ("converted (object accessors)"), or a
# builtin names a raw block pointer (`*const Block`, `vm_blk`, `BlkArg`, `.ptr()`, `as *const _`, an address-to-pointer fn).
# Generated sources (src/node_*.rs, src/prism_ast.rs) are not counted; node.def is.
ROOT = File.expand_path('..', __dir__)
BODY = %w[obj rary rstr rhash rclass rproc rmethod rbig rdata ivars_of str_bytes]
ARY  = %w[ary_new ary_new_capa ary_new_from ary_from_vec ary_push ary_len ary_get ary_to_vec]
ALLOC = %w[str_new str_from_vec str_from_vec_enc hash_new float_new]
# context-free API of hash.rs, range.rs, numeric.rs, bignum.rs (heap::no_legacy_hn)
HN = %w[hash_new_capa hash_pairs hash_len hash_entry_at hash_clear hash_dup range_new range_parts num_to_f64 big_to_value to_big int_from_i64 int_from_i128 rfloat_heap]
# names of the former context-free String API (str_legacy.rs, gone), banned where heap::no_legacy_str is imported
STR  = %w[str_new_enc str_with_capa str_new_frozen str_enc str_eq str_to_sym str_append_same_enc char_offsets str_char_len str_dup ary_of_strs]
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
  body_only = !converted && src =~ /^\s*use crate::heap::no_legacy_body::\*/ ? true : false
  helpers = src.scan(/\bfn\s+\w+[^{;]*->[^{;]*&'static mut/).size   # local accessors handing out &'static mut
  if converted
    banned = BODY + ARY + ALLOC + %w[vm]
    banned += HN if src =~ /^\s*use crate::heap::no_legacy_hn::\*/
    banned += STR if src =~ /^\s*use crate::heap::no_legacy_str::\*/
    left = banned.select { count(body, _1) > 0 || body =~ /::#{_1}\(/ }
    bad << "#{rel}: #{left.join(' ')}" unless left.empty?
  end
  if body_only
    left = BODY.select { count(body, _1) > 0 || body =~ /::#{_1}\(/ }
    bad << "#{rel}: #{left.join(' ')}" unless left.empty?
  end
  # a builtin holds its block as a Blk<'call> (vm.rs): it may not name the raw type, make one from an
  # address, or reach the VM's raw constructor; `// blk-audit:` on the line states why one may
  if rel.start_with?('src/builtins/')
    raw.each_line.with_index(1) do |l, i|
      next if l.include?('// blk-audit:')
      code = l.sub(%r{//.*}, '')
      bad << "#{rel}:#{i}: raw block (#{$&})" if code =~ /\*const Block|\bvm_blk\b|\bBlkArg\b|\.ptr\(\)|as \*const _|\b(without_provenance|with_exposed_provenance|with_addr|map_addr)\b/
    end
  end
  statics = src.scan(/\bstatic\s+mut\s+\w+\s*:/).size
  rows << [rel, c, uns, converted ? true : (body_only ? :body : false), raw.include?('#![forbid(unsafe_code)]'), helpers, statics]
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
  state = conv == :body ? 'converted (object accessors)' : conv ? (forbid ? 'converted, forbid(unsafe_code)' : 'converted') : (forbid ? 'forbid(unsafe_code)' : '')
  puts fmt.([md ? rel : rel.ljust(w), *heads.map { c[_1].to_s.rjust(6) }, s.to_s.rjust(5), helpers.to_s.rjust(16), statics.to_s.rjust(10), uns.to_s.rjust(6), state])
end
puts fmt.([md ? 'total' : 'total'.ljust(w), *heads.map { tot[_1].to_s.rjust(6) }, tot.values.sum.to_s.rjust(5), th.to_s.rjust(16), ts.to_s.rjust(10), tu.to_s.rjust(6), ''])
unless bad.empty?
  warn "converted files still naming old accessors, or builtins holding a raw block:\n  " + bad.join("\n  ")
  exit 1
end
