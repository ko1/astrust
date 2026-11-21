#!/usr/bin/env ruby
# unsafe_catalog.rb [--md] [--list]: every `unsafe` of saruby's hand-written sources, by category
# (docs/unsafe_catalog.md).  Counted as tools/heap_audit.rb counts them (line comments stripped, one per
# token: `unsafe fn`, `unsafe impl`, `#[unsafe(naked)]` and `unsafe {}` each count), so the totals agree.
# The category is decided by the file, and inside the call-path files by the text of the line (and the
# five lines after it when the line opens a block): a heuristic, first match wins; --list prints the
# category of every occurrence so that it can be checked.  Generated sources (src/node_*.rs,
# src/prism_ast.rs) are reported as one total each.
ROOT = File.expand_path('..', __dir__)
md = ARGV.delete('--md'); list = ARGV.delete('--list')
def strip(s) = s.gsub(%r{//[^\n]*}, '')

BY_FILE = {
  'src/builtins/sys.rs' => 'OS interface: libc wrappers',
  'src/kalloc.rs' => 'OS interface: global allocator',
  'src/fiber_os.rs' => 'fiber: stacks and context switch',
  'src/fiber.rs' => 'fiber: stacks and context switch',
  'src/aot.rs' => 'code loading: code store, patch binding',
  'src/prism.rs' => 'parser FFI (prism)',
  'src/compile.rs' => 'node building: raw node reads',
  'src/sbuf.rs' => 'String buffer (SBuf)',
  'src/stack_raw.rs' => 'value stack, raw backend (FEATURES=raw_stack only)',
  'src/lib.rs' => 'GC & heap base',
  'src/heap.rs' => 'GC & heap base',
  'src/value.rs' => 'GC & heap base',
  'src/gc.rs' => 'GC & heap base',
  'src/object.rs' => 'GC & heap base: legacy accessors',
}
CALL_PATH = ['src/vm.rs', 'src/rt.rs', 'node.def']
RULES = [
  [/static mut|REFINEMENTS_ON|BOOTED|VM\.as_(mut|deref)|HEAP\.as_mut|STACK_BASE|STRESS/, 'static mut global'],
  [/visit_ctx|visit_ambiguous|m\.visit|main_ctx/, 'GC root scan'],
  [/transmute|cc\.fast as|cc\.poly|yc\.seen|cc\.val/, 'cache word reinterpreted'],
  [/unreachable_unchecked/, 'unchecked kind assumption'],
  [/from_raw|from_ptr/, 'Value from a raw word'],
  [/head\.(kind|dispatcher)|NodeOf/, 'node pointer'],
  [/Pos::bound|const unsafe fn bound|\*self\.0 \}/, 'line position (Pos, patch holes)'],
  [/FrameFull|c\.frame|self\.frame|self\.prev|\(\*g\)|\(\*pf\)|frame\.as_ref|f\.prev|\(\*f\b|&\*f\b|\(\*self\.0\)|\*self\.0/, 'frame chain'],
  [/\(\*(me|mm|rm|im|fme)\)|&\*me\b|\(\*fr\.me\)|\(\*self\.me\)|&\*f\.me|me as \*mut MethodEntry/, 'method entry (leaked Box)'],
  [/\(\*is\)|&\*is\b|iseq\(\)|\(\*iseq\)|&\*iseq/, 'iseq (never freed)'],
  [/blk|\(\*b\)|&\*b\b|outer_blk/, 'block pointer (H4)'],
  [/\(\*cr\)|\(\*cref\)|cr as \*mut Cref/, 'cref chain (leaked)'],
]
HEAP_RULES = [
  [/mint\(\)|Token\(/, 'context token (Token::mint)'],
  [/VM\.as_|HEAP\.as_mut|static mut|STACK_BASE|STRESS|BOOTED|&raw mut crate::vm::VM/, 'static mut global'],
  [/from_raw|from_ptr/, 'Value from a raw word'],
  [/\(\*robj\(v\)\)\.(klass|flags)/, 'object header place access'],
  [/frame\.as_ref|f\.prev|\.prev\.as_mut|set_cref/, 'frame chain (H5)'],
  [/me\.as_ref|cr\.as_ref|is\.as_ref|blk\.as_ref|MethodEntry|Cref|lookup_method|is\.as_ref|REFINEMENTS_ON/, 'entry / iseq / block / cref reads (leaked, H4)'],
  [/\(\*robj\(v\)\)\.body|\(\*v\.ptr\(\)\)\.body/, 'payload accessor (body / body_mut / *_mut_and / kind test)'],
  [/GcLang|finalize|drop_body|new_unchecked|with_current_stack|drop_in_place/, 'GC binding (gc.rs)'],
  [/unreachable_unchecked|&mut \*v\.ptr\(\)/, 'legacy accessor (&\'static mut, unchecked kind)'],
]

occ = []   # [file, line, category]
files = Dir[File.join(ROOT, 'src/**/*.rs')].reject { _1 =~ %r{/src/(node_\w+|prism_ast)\.rs\z} }.sort + [File.join(ROOT, 'node.def')]
gen = Dir[File.join(ROOT, 'src/{node_*,prism_ast}.rs')].sort
files.each do |f|
  rel = f.sub(ROOT + '/', '')
  lines = File.readlines(f).map { strip(_1) }
  lines.each_with_index do |l, i|
    n = l.scan(/\bunsafe\b/).size
    next if n == 0
    ctx = l.rstrip.end_with?('{') ? lines[i, 6].join : l
    cat = if CALL_PATH.include?(rel)
      'call path: ' + (RULES.find { |re, _| ctx =~ re }&.last || 'other')
    elsif BY_FILE[rel]&.start_with?('GC & heap base') && rel != 'src/object.rs'
      'GC & heap base: ' + (HEAP_RULES.find { |re, _| ctx =~ re }&.last || 'other')
    elsif l =~ /format!\(/ then "#{BY_FILE[rel] || rel} (text emitted into generated code)"
    else BY_FILE[rel] || "unclassified: #{rel}"
    end
    n.times { occ << [rel, i + 1, cat] }
  end
end

if list
  occ.each { |f, l, c| puts "#{f}:#{l}\t#{c}" }
  exit
end
by = occ.group_by { _3 }.transform_values(&:size).sort_by { |k, v| [k.split(':').first, -v] }
w = by.map { _1[0].size }.max
fmt = md ? ->(a, b) { "| #{a} | #{b} |" } : ->(a, b) { "#{a.ljust(w)} #{b.to_s.rjust(5)}" }
puts fmt.('category', 'unsafe'); puts '|---|---:|' if md
by.each { |k, v| puts fmt.(k, v) }
puts fmt.('total (hand-written, = heap_audit.rb total)', occ.size)
gen.each { |f| puts fmt.("generated: #{f.sub(ROOT + '/', '')}", strip(File.read(f)).scan(/\bunsafe\b/).size) }
