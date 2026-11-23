#!/usr/bin/env ruby
# gen_unsafe_catalog.rb [--md] [--root SAMPLE_DIR] [CODE_STORE_DIR...]: every `unsafe` of the generated sources (src/node_*.rs) and,
# for each code store given, of its baked crates (all.rs, patch.rs), by the astrustgen template that emits it.
# Counted as tools/unsafe_catalog.rb counts (line comments stripped, one per `unsafe` token, string literals
# included: node_specialize.rs holds the text it emits).  The template is decided by the file, the enclosing
# fn (generated sources) or item (baked crates) and the text of the line, first match wins.  For the unsafe
# blocks of baked T_ items it also counts what they need unsafe for (ops), per occurrence in the block's text.
ROOT = (i = ARGV.index('--root')) ? File.expand_path(ARGV.slice!(i, 2)[1]) : File.expand_path('..', __dir__)
md = ARGV.delete('--md')
def strip(s) = s.gsub(%r{//[^\n]*}, '')

# [file pattern, fn pattern (nil: any), line pattern, template]
GEN_RULES = [
  ['head', /\AALLOC_/, /head\.\w+ = /, 'ALLOC: head-hole store'],
  ['head', /\ACHILDREN_/, /&mut \(\*\(n as/, 'CHILDREN: &mut operand struct from NodePtr'],
  ['head', /\ACHILDREN_/, /p\.add/, 'CHILDREN: list element &mut through the shared slice'],
  ['head', /\ACHILDREN_/, /node_u_mut/, 'CHILDREN: operand struct (node_u_mut)'],
  ['head', /\A(Dyn|DynList|call|nth|each)\z/, /head\.dispatcher\)\(|node_dispatcher/, 'Dyn / DynList: call through the head dispatcher'],
  ['head', nil, /head\.kind\.\w+\)\(/, 'helper: call through the kind table'],
  ['head', /\Alayout_tree|\Asize\z|\Acopy\z/, //, 'layout_tree: copy into one block'],
  ['head', /\Alist_base/, //, 'list_base: empty slice at the parent'],
  ['head', /\Anode_(u|u_mut|kind|flags|set_flags|hash_get|hash_set|dispatcher|head_\w+)\z/, //, 'primitive: node access (audited)'],
  ['head', /\A(body_mut|deref)\z/, //, 'primitive: @ref cell (audited)'],
  ['head', /\A(hash_node|specialize_node|specialize_open|node_laid)\z/, //, 'helper: head flags / hash / kind'],
  ['dispatch', nil, //, 'DISPATCH: glue block (field reads, &mut @ref, head hole)'],
  ['hash', /\AHASH_/, /let u = /, 'HASH: operand struct from NodePtr'],
  ['hash', /\APROF_/, /let u = /, 'PROF: operand struct from NodePtr'],
  ['hash', /\APROF_/, /let p: NodePtr/, 'PROF: @spec target hook'],
  ['hash', /\APROF_/, /hash_u64/, 'PROF: @bake value hook'],
  ['hash', nil, //, 'HASH: embedder opaque-operand hook'],
  ['dump', nil, //, 'DUMP: field reads'],
  ['replace', nil, /p\.add/, 'REPLACE: list element write through the shared slice'],
  ['replace', nil, //, 'REPLACE: &mut operand struct from NodePtr'],
  ['specialize', nil, /hole_sym|H_%SYM%|asm!|Pos::bound/, 'emitted: hole patch item (embedder hook text)'],
  ['specialize', nil, /push_str\(&format!\(".*(fn (call|test|spec)|unsafe \{\{)/, 'emitted: SD call / test / spec block'],
  ['specialize', /\Aspecialize_list/, //, 'emitted: L_ nth / each'],
  ['specialize', nil, /let u = /, 'SPECIALIZE: operand struct from NodePtr'],
  ['specialize', nil, /let p: NodePtr/, 'SPECIALIZE: @spec target hook'],
  ['specialize', nil, /\A\s*unsafe \{\s*\z/, 'SPECIALIZE: argument block (field reads at bake time)'],
  ['free', nil, //, 'FREE: Box::from_raw (embedder task, astrogre)'],
  ['eval', nil, //, 'EVAL: node.def body (hand-written, counted there)'],
]
BAKED_RULES = [
  [/\AH_/, /asm!|Pos::bound/, 'H_: hole patch item'],
  [/\AL_/, /unreachable_unchecked/, 'L_ nth: unreachable arm'],
  [/\AL_/, /byte_offset/, 'L_ nth / each: child at an offset'],
  [/\AT_/, /fn (call|test|spec)\(/, 'T_: call / test / spec block'],
]
OPS = {
  'field read (*np).u.x' => /(?<!&mut )\(\*np\)\.u\.\w+/,
  '&mut @ref (*np).u.x' => /&mut \(\*np\)\.u\.\w+/,
  'head hole (*np).head.x' => /\(\*np\)\.head\.\w+/,
  'child n.byte_offset(K)' => /\bn\.byte_offset\(/,
}

def classify_gen(path)
  rel = File.basename(path, '.rs').sub(/\Anode_/, '')
  fn = ''
  out = Hash.new(0)
  strip(File.read(path)).each_line do |l|
    fn = $1 if l =~ /\A\s*(?:pub )?fn (\w+)/
    k = l.scan(/\bunsafe\b/).size
    next if k == 0
    r = GEN_RULES.find { |f, fr, lr, _| f == rel && (fr.nil? || fn =~ fr) && l =~ lr }
    out[r ? r[3] : "#{rel}: other (#{fn})"] += k
  end
  out
end

def classify_baked(path)
  item = ''
  out = Hash.new(0); ops = Hash.new(0); blocks = []; cur = nil
  strip(File.read(path)).each_line do |l|
    item = $1 if l =~ /\Apub (?:struct|fn|static) (\w+)/
    if cur
      cur << l
      if l =~ /\A    \} \}\s*\z/ then blocks << cur; cur = nil end
    elsif item.start_with?('T_') && l =~ /\{ unsafe \{\s*\z/
      cur = +l
    end
    k = l.scan(/\bunsafe\b/).size
    next if k == 0
    r = BAKED_RULES.find { |ir, lr, _| item =~ ir && l =~ lr }
    out[r ? r[2] : "other (#{item[0, 2]})"] += k
  end
  blocks.each { |b| OPS.each { |name, re| ops[name] += b.scan(re).size } }
  items = strip(File.read(path)).scan(/^pub (?:struct|fn) ([A-Z]+)_/).flatten.tally
  [out, ops, items]
end

fmt = md ? ->(*a) { "| #{a.join(' | ')} |" } : ->(*a) { a[0].ljust(62) + a[1..].map { _1.to_s.rjust(7) }.join }
gen = Dir[File.join(ROOT, 'src/node_*.rs')].sort
tot = 0
puts fmt.('generated source / template', 'unsafe'); puts '|---|---:|' if md
gen.each do |f|
  c = classify_gen(f)
  s = c.values.sum; tot += s
  puts fmt.("**#{File.basename(f)}**", s) if md
  puts fmt.(File.basename(f), s) unless md
  c.sort_by { -_2 }.each { |k, v| puts fmt.("  #{k}", v) }
end
puts fmt.('total (generated sources)', tot)
ARGV.each do |dir|
  %w[all.rs patch.rs].each do |b|
    f = File.join(dir, b)
    next unless File.exist?(f)
    c, ops, items = classify_baked(f)
    puts
    puts fmt.("#{dir.sub(Dir.home, '~')}/#{b}", 'unsafe'); puts '|---|---:|' if md
    c.sort_by { -_2 }.each { |k, v| puts fmt.("  #{k}", v) }
    puts fmt.('  total', c.values.sum)
    puts fmt.('  items: ' + items.sort.map { |k, v| "#{k}_ #{v}" }.join(', '), '')
    ops.each { |k, v| puts fmt.("  op in T_ blocks: #{k}", v) }
  end
end
