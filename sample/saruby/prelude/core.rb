# saruby prelude: built-ins written in Ruby (loaded at boot).
module Comparable
  def ==(o)
    return true if equal?(o)
    seen = __saruby__.__ivget(Comparable, :__eqrec) || __saruby__.__ivset(Comparable, :__eqrec, {})
    key = [__id__, o.__id__]
    return false if seen.key?(key)
    seen[key] = true
    begin
      r = (self <=> o)
    ensure
      seen.delete(key)
    end
    r.nil? ? false : __saruby__.__cmpint(r) == 0
  end
end

module Enumerable
  # __saruby__.__each(self) { |v| } packs a multi-value yield into one element (CRuby's ENUM_WANT_SVALUE)
  # without Ruby-level calls; methods that hand raw values to the block use each { |*x| yield(*x) }.
  def grep(pat)
    a = []
    if block_given?
      __saruby__.__each(self) { |v| a << yield(v) if pat === v }
    else
      # without a block the caller's $~ stays (CRuby's rb_reg_match_p); this frame shares it (svar.rs)
      lm = $~
      __saruby__.__each(self) { |v| a << v if pat === v }
      $~ = lm
    end
    a
  end
  def grep_v(pat)
    a = []
    if block_given?
      __saruby__.__each(self) { |v| a << yield(v) unless pat === v }
    else
      lm = $~
      __saruby__.__each(self) { |v| a << v unless pat === v }
      $~ = lm
    end
    a
  end
  def find_index(*args)
    __saruby__.__argc(args.size, 1, 1) if args.size > 1
    if args.empty?
      return enum_for(:find_index) unless block_given?
      i = 0
      each { |*x| return i if yield(*x); i += 1 }
    else
      warn("given block not used", uplevel: 1) if block_given?
      i = 0
      __saruby__.__each(self) { |v| return i if v == args[0]; i += 1 }
    end
    nil
  end
  def each_entry(*args)
    return enum_for(:each_entry, *args) { __saruby__.__size(self) } unless block_given?
    __saruby__.__each(self, *args) { |v| yield v }
    self
  end
  def sum(init = 0, &b)
    return __saruby__.__range_sum(self, init) if !b && Range === self
    a = []
    if b
      __saruby__.__each(self) { |v| a << b.call(v) }
    else
      __saruby__.__each(self) { |v| a << v }
    end
    a.sum(init)
  end
  def take_while
    return enum_for(:take_while) unless block_given?
    a = []
    each { |*x| break unless yield(*x); a << (x.size <= 1 ? x[0] : x) }
    a
  end
  def drop(n)
    n = __saruby__.__int(n)
    raise ArgumentError, "attempt to drop negative size" if n < 0
    a = []; i = 0
    __saruby__.__each(self) { |v| a << v if i >= n; i += 1 }
    a
  end
  def drop_while
    return enum_for(:drop_while) unless block_given?
    a = []; dropping = true
    __saruby__.__each(self) { |v| dropping = false if dropping && !yield(v); a << v unless dropping }
    a
  end
  def sort(&b) = __saruby__.__to_a(self).sort(&b)
  def chunk_while(&b)
    raise ArgumentError, "tried to create Proc object without a block" unless b
    src = self
    Enumerator.new do |y|
      cur = nil
      __saruby__.__each(src) do |v|
        if cur.nil? then cur = [v]
        elsif b.call(cur.last, v) then cur << v
        else y << cur; cur = [v] end
      end
      y << cur if cur
    end
  end
  def slice_when(&b)
    raise ArgumentError, "tried to create Proc object without a block" unless b
    chunk_while { |a, c| !b.call(a, c) }
  end
  def chunk(&b)
    return enum_for(:chunk) { __saruby__.__size(self) } unless b
    src = self
    Enumerator.new do |y|
      ck = nil; cur = nil
      __saruby__.__each(src) do |v|
        k = b.call(v)
        if k.nil? || k == :_separator
          y << [ck, cur] if cur
          ck = cur = nil
        elsif k == :_alone
          y << [ck, cur] if cur
          y << [k, [v]]
          ck = cur = nil
        elsif k.is_a?(Symbol) && k.start_with?("_")
          raise RuntimeError, "symbols beginning with an underscore are reserved"
        elsif cur && ck == k
          cur << v
        else
          y << [ck, cur] if cur
          ck = k; cur = [v]
        end
      end
      y << [ck, cur] if cur
    end
  end
  def slice_before(*pat, &b)
    if b
      raise ArgumentError, "both pattern and block are given" unless pat.empty?
    else
      __saruby__.__argc(pat.size, 1, 1) if pat.size != 1
    end
    pat = pat[0]
    src = self
    Enumerator.new do |y|
      cur = nil
      __saruby__.__each(src) do |v|
        hit = b ? b.call(v) : (pat === v)
        if cur && hit then y << cur; cur = nil end
        (cur ||= []) << v
      end
      y << cur if cur
    end
  end
  def slice_after(*pat, &b)
    if b
      raise ArgumentError, "both pattern and block are given" unless pat.empty?
    else
      __saruby__.__argc(pat.size, 1, 1) if pat.size != 1
    end
    pat = pat[0]
    src = self
    Enumerator.new do |y|
      cur = nil
      __saruby__.__each(src) do |v|
        (cur ||= []) << v
        if b ? b.call(v) : (pat === v) then y << cur; cur = nil end
      end
      y << cur if cur
    end
  end
  def all?(*pat)
    __saruby__.__argc(pat.size, 0, 1) if pat.size > 1
    warn("given block not used", uplevel: 1) if !pat.empty? && block_given?
    if !pat.empty? then __saruby__.__each(self) { |v| return false unless pat[0] === v }
    elsif block_given? then each { |*x| return false unless yield(*x) }
    else __saruby__.__each(self) { |v| return false unless v } end
    true
  end
  def any?(*pat)
    __saruby__.__argc(pat.size, 0, 1) if pat.size > 1
    warn("given block not used", uplevel: 1) if !pat.empty? && block_given?
    if !pat.empty? then __saruby__.__each(self) { |v| return true if pat[0] === v }
    elsif block_given? then each { |*x| return true if yield(*x) }
    else __saruby__.__each(self) { |v| return true if v } end
    false
  end
  def none?(*pat)
    __saruby__.__argc(pat.size, 0, 1) if pat.size > 1
    warn("given block not used", uplevel: 1) if !pat.empty? && block_given?
    if !pat.empty? then __saruby__.__each(self) { |v| return false if pat[0] === v }
    elsif block_given? then each { |*x| return false if yield(*x) }
    else __saruby__.__each(self) { |v| return false if v } end
    true
  end
  def one?(*pat)
    __saruby__.__argc(pat.size, 0, 1) if pat.size > 1
    warn("given block not used", uplevel: 1) if !pat.empty? && block_given?
    n = 0
    if !pat.empty? then __saruby__.__each(self) { |v| n += 1 if pat[0] === v; return false if n > 1 }
    elsif block_given? then each { |*x| n += 1 if yield(*x); return false if n > 1 }
    else __saruby__.__each(self) { |v| n += 1 if v; return false if n > 1 } end
    n == 1
  end
  def zip(*others, &b)
    a = to_a
    others = others.map do |o|
      if (oa = Array.try_convert(o)) then oa
      elsif o.respond_to?(:each) then o.to_enum(:each).first(a.size)
      else raise TypeError, "wrong argument type #{o.class} (must respond to :each)"
      end
    end
    a.zip(*others, &b)
  end
  def uniq
    h = {}
    if block_given?
      each { |*x| k = yield(*x); h[k] = (x.size <= 1 ? x[0] : x) unless h.key?(k) }
    else
      __saruby__.__each(self) { |v| h[v] = v unless h.key?(v) }
    end
    h.values
  end
  def to_h(*args, &b)
    h = {}
    i = 0
    each(*args) do |*x|
      pair = b ? yield(*x) : (x.size <= 1 ? x[0] : x)
      ar = Array.try_convert(pair)
      raise TypeError, "wrong element type #{pair.class} (expected array)" unless ar
      raise ArgumentError, "element has wrong array length (expected 2, was #{ar.size})" if ar.size != 2
      h[ar[0]] = ar[1]
      i += 1
    end
    h
  end
  def to_set(*args, &b)
    warn("passing arguments to Enumerable#to_set is deprecated", uplevel: 1) unless args.empty?
    klass = args.empty? ? Set : args.shift
    klass.new(self, *args, &b)
  end
  def reverse_each(*args, &b)
    return enum_for(:reverse_each, *args) { __saruby__.__size(self) } unless b
    to_a(*args).reverse_each(&b)
    self
  end
  def cycle(*args)
    __saruby__.__argc(args.size, 0, 1) if args.size > 1
    n = args[0]
    unless block_given?
      return enum_for(:cycle, *args) do
        if n.nil? then (s = __saruby__.__size(self)).nil? ? nil : s == 0 ? 0 : Float::INFINITY
        else s = __saruby__.__size(self); k = __saruby__.__int(n); s && (k < 0 ? 0 : s * k) end
      end
    end
    unless n.nil?
      n = __saruby__.__int(n)
      return nil if n <= 0
    end
    saved = []
    __saruby__.__each(self) { |v| saved << v; yield v }
    return nil if saved.empty?
    if n.nil?
      loop { saved.each { |v| yield v } }
    else
      (n - 1).times { saved.each { |v| yield v } }
    end
    nil
  end
  def chain(*enums) = __saruby__.__chain([self, *enums])
  def compact
    a = []
    __saruby__.__each(self) { |v| a << v unless v.nil? }
    a
  end
end


module GC
  # CRuby's gc.rb signature: keywords, no Hash allocated per call
  def self.start(full_mark: true, immediate_mark: true, immediate_sweep: true) = __saruby__.__gc_start(full_mark)
end

module Kernel
  # CRuby's prelude.rb: lib/pp.rb (prelude/lib/pp.rb) loads on the first call and redefines pp
  def pp(*objs)
    require 'pp'
    pp(*objs)
  end
  alias pp pp
  private :pp
end

module Kernel
  # pathname_builtin.rb; class Pathname itself loads on first reference (boot.rs lazy_const)
  def Pathname(path)
    return path if Pathname === path
    Pathname.new(path)
  end
  module_function :Pathname
end

class NilClass
  def to_a = []
  def to_h = {}
  def &(o) = false
  def |(o) = !!o
  alias ^ |
  def nil? = true
  def to_i = 0
  def to_f = 0.0
  def =~(o) = nil
  def ===(o) = equal?(o)
end
class TrueClass
  alias inspect to_s
  def &(o) = !!o
  def |(o) = true
  def ^(o) = !o
  def ===(o) = equal?(o)
end
class FalseClass
  alias inspect to_s
  def &(o) = false
  def |(o) = !!o
  alias ^ |
  def ===(o) = equal?(o)
end

class Integer
  def numerator = self
  def denominator = 1
end

class Struct
end

class Enumerator
  def +(other) = __saruby__.__chain([self, other])
  def self.produce(*args, **kw, &b)
    raise ArgumentError, "no block given" unless b
    # RB_SCAN_ARGS_LAST_HASH_KEYWORDS: a trailing Hash is the keywords
    kw = args.pop if kw.empty? && Hash === args.last
    __saruby__.__argc(args.size, 0, 1) if args.size > 1
    __saruby__.__kw(kw, :size) unless kw.empty?
    Enumerator.new(kw.fetch(:size, Float::INFINITY)) do |y|
      v = args.empty? ? b.call(nil) : args[0]
      begin
        loop { y << v; v = b.call(v) }
      rescue StopIteration
      end
    end
  end
  def self.product(*enums, **kw, &b)
    unless kw.empty?
      ks = kw.keys.map(&:inspect).join(", ")
      raise ArgumentError, "unknown keyword#{kw.size > 1 ? 's' : ''}: #{ks}"
    end
    e = Enumerator::Product.new(*enums)
    return e unless b
    e.each(&b)
    nil
  end

  class Generator
    def initialize(*args, &b)
      __saruby__.__argc(args.size, 0, 1) if args.size > 1
      unless args.empty?
        raise TypeError, "wrong argument type #{args[0].class} (expected Proc)" unless args[0].is_a?(Proc)
        b = args[0]
      end
      raise LocalJumpError, "no block given" unless b
      @__b = b
    end
    def each(*args, &blk) = @__b.call(Enumerator::Yielder.new(&blk), *args)
  end

  class Chain < Enumerator
    undef_method :feed, :next, :next_values, :peek, :peek_values
    def initialize(*enums)
      @__e = enums.freeze
      self
    end
    def each(*args, &b)
      return self unless b
      @__e.each_with_index { |e, i| __saruby__.__ivset(self, :__p, i); e.each(*args, &b) }
      self
    end
    def size
      @__e.inject(0) do |t, e|
        s = e.size
        return s if s.nil? || (s.is_a?(Float) && s.infinite?)
        t + s
      end
    end
    # enum_chain_rewind: the ones iterated so far, last first
    def rewind
      i = @__p || -1
      en = @__e
      while i >= 0 && i < en.size
        en[i].rewind if en[i].respond_to?(:rewind)
        __saruby__.__ivset(self, :__p, (i -= 1))
      end
      self
    end
    def inspect
      en = @__e or return "#<#{self.class}: uninitialized>"
      seen = (__saruby__.__ivget(Enumerator::Chain, :__insp) || __saruby__.__ivset(Enumerator::Chain, :__insp, {}))
      return "#<#{self.class}: ...>" if seen.key?(object_id)
      seen[object_id] = true
      begin
        "#<#{self.class}: #{en.inspect}>"
      ensure
        seen.delete(object_id)
      end
    end
  end
end

class Set
  include Enumerable
  def self.[](*a) = new(a)
  def initialize(enum = nil, &b)
    __saruby__.__ivset(self, :__h, {})
    return if enum.nil?
    raise ArgumentError, "value must be enumerable" unless enum.respond_to?(:each_entry) || enum.respond_to?(:each)
    __saruby__.__set_init(@__h, enum, &b)
  end
  def initialize_copy(o)
    super
    __saruby__.__ivset(self, :__h, __saruby__.__ivget(o, :__h).dup)
  end
  def compare_by_identity
    __saruby__.set_frozen!(self)
    @__h.compare_by_identity
    self
  end
  def compare_by_identity? = @__h.compare_by_identity?
  def add(x)
    __saruby__.set_frozen!(self)
    h = @__h
    if @__it && !h.key?(x)
      raise RuntimeError, "can't add a new item into set during iteration"
    end
    h[x] = true
    self
  end
  alias << add
  def add?(x) = (@__h.key?(x) ? nil : add(x))
  def delete(x)
    __saruby__.set_frozen!(self)
    @__h.delete(x)
    self
  end
  def delete?(x) = (@__h.key?(x) ? delete(x) : nil)
  def include?(x) = @__h.key?(x)
  alias member? include?
  alias === include?
  def size = @__h.size
  alias length size
  def empty? = @__h.empty?
  def each(&b)
    return enum_for(:each) { size } unless b
    it = @__it
    __saruby__.__ivset(self, :__it, true)
    begin
      @__h.each_key(&b)
    ensure
      __saruby__.__ivset(self, :__it, it)
    end
    self
  end
  def to_a = @__h.keys
  def to_set(*a, &b) = (a.empty? && !b && instance_of?(Set) ? self : Set.new(self, *a, &b))
  def |(o) = (s = dup; s.merge(__saruby__.set_enum(o)); s)
  alias union |
  alias + |
  def -(o) = (s = dup; s.subtract(__saruby__.set_enum(o)); s)
  alias difference -
  def &(o)
    s = __saruby__.__set_like(self)
    __saruby__.set_enum(o).each { |x| s.add(x) if include?(x) }
    s
  end
  alias intersection &
  def ^(o)
    s = dup
    unless o.is_a?(Set)
      t = __saruby__.__set_like(self)
      __saruby__.__set_init(__saruby__.__ivget(t, :__h), __saruby__.set_enum(o))
      o = t
    end
    o.each { |x| s.add(x) unless s.delete?(x) }
    s
  end
  def subset?(o) = (__saruby__.set_set(o); size <= o.size && all? { |x| o.include?(x) })
  alias <= subset?
  def superset?(o) = (__saruby__.set_set(o); size >= o.size && o.all? { |x| include?(x) })
  alias >= superset?
  def proper_subset?(o) = (__saruby__.set_set(o); size < o.size && all? { |x| o.include?(x) })
  alias < proper_subset?
  def proper_superset?(o) = (__saruby__.set_set(o); size > o.size && o.all? { |x| include?(x) })
  alias > proper_superset?
  def <=>(o)
    return nil unless o.is_a?(Set)
    return 0 if self == o
    return -1 if proper_subset?(o)
    return 1 if proper_superset?(o)
    nil
  end
  def disjoint?(o) = !intersect?(o)
  def intersect?(o)
    o = __saruby__.set_enum(o)
    if o.is_a?(Set) && o.size < size then o.any? { |x| include?(x) }
    elsif o.is_a?(Set) then any? { |x| o.include?(x) }
    else o.any? { |x| include?(x) }
    end
  end
  def merge(*es)
    raise RuntimeError, "cannot add to set during iteration" if @__it
    __saruby__.set_frozen!(self)
    __saruby__.__set_merge(@__h, es)
    self
  end
  def subtract(e) = (__saruby__.set_enum(e).each { |x| delete(x) }; self)
  def replace(e)
    e = __saruby__.set_enum(e)
    __saruby__.set_frozen!(self)
    raise RuntimeError, "cannot replace set during iteration" if @__it
    # a Set is copied with its compare_by_identity (set_i_initialize_copy)
    return (__saruby__.__ivset(self, :__h, __saruby__.__ivget(e, :__h).dup); self) if e.is_a?(Set)
    a = e.to_a
    @__h.clear
    a.each { |x| add(x) }
    self
  end
  def ==(o)
    return true if equal?(o)
    return false unless o.is_a?(Set)
    __saruby__.__set_eq(@__h, __saruby__.__ivget(o, :__h))
  end
  def hash = to_a.map(&:hash).sort.hash
  alias eql? ==
  def clear = (__saruby__.set_frozen!(self); @__h.clear; self)
  def reset = (__saruby__.set_frozen!(self); @__h.rehash; self)
  def collect!(&b)
    return enum_for(:collect!) { size } unless b
    a = to_a.map(&b)
    __saruby__.set_frozen!(self)
    @__h.clear
    a.each { |x| add(x) }
    self
  end
  alias map! collect!
  def select!(&b)
    return enum_for(:select!) { size } unless b
    __saruby__.set_frozen!(self)
    n = size; @__h.select! { |k, _| b.call(k) }; n == size ? nil : self
  end
  alias filter! select!
  def reject!(&b)
    return enum_for(:reject!) { size } unless b
    __saruby__.set_frozen!(self)
    n = size; @__h.reject! { |k, _| b.call(k) }; n == size ? nil : self
  end
  def delete_if(&b) = (return enum_for(:delete_if) { size } unless b; reject!(&b); self)
  def keep_if(&b) = (return enum_for(:keep_if) { size } unless b; select!(&b); self)
  def classify
    return enum_for(:classify) { size } unless block_given?
    h = {}
    each { |x| (h[yield(x)] ||= __saruby__.__set_like(self)).add(x) }
    h
  end
  def divide(&b)
    return enum_for(:divide) { size } unless b
    if b.arity == 2
      items = to_a
      parent = {}
      find = ->(x) { x = parent[x] while parent[x] != x; x }
      items.each { |x| parent[x] = x }
      items.each_with_index { |a, i| items[(i + 1)..].each { |c| parent[find.(a)] = find.(c) if b.call(a, c) && b.call(c, a) && find.(a) != find.(c) } }
      groups = {}
      items.each { |x| (groups[find.(x)] ||= __saruby__.__set_like(self)).add(x) }
      s = Set.new; groups.each_value { |g| s.add(g) }; s
    else
      s = Set.new; classify(&b).each_value { |g| s.add(g) }; s
    end
  end
  def flatten = __saruby__.set_flatten_into(self, __saruby__.__set_like(self), { object_id => true })
  def flatten!
    return nil unless any? { |x| x.is_a?(Set) }
    replace(flatten)
  end
  def join(sep = nil) = to_a.join(sep)
  def inspect
    seen = (__saruby__.__ivget(Set, :__insp) || __saruby__.__ivset(Set, :__insp, {}))
    sub = !instance_of?(Set)
    if seen.key?(object_id)
      return sub ? "#<#{self.class}: {...}>" : "#{self.class}[...]"
    end
    seen[object_id] = true
    begin
      body = to_a.map(&:inspect).join(", ")
      sub ? "#<#{self.class}: {#{body}}>" : "#{self.class}[#{body}]"
    ensure
      seen.delete(object_id)
    end
  end
  alias to_s inspect
end


# the prelude's own helpers, on the internal module (no constant names it)
def __saruby__.__size(o) = (o.respond_to?(:size) ? o.size : nil)
def __saruby__.set_enum(o)
  return o if o.is_a?(Set)
  raise ArgumentError, "value must be enumerable" unless o.respond_to?(:each_entry) || o.respond_to?(:each)
  o
end
def __saruby__.set_set(o)
  raise ArgumentError, "value must be a set" unless o.is_a?(Set)
  o
end
def __saruby__.set_frozen!(s) = (raise FrozenError.new("can't modify frozen #{s.class}: #{s.inspect}", receiver: s) if s.frozen?)
def __saruby__.set_flatten_into(s, out, seen)
  s.each do |x|
    if x.is_a?(Set)
      raise ArgumentError, "tried to flatten recursive Set" if seen[x.object_id]
      seen[x.object_id] = true
      set_flatten_into(x, out, seen)
      seen.delete(x.object_id)
    else
      out.add(x)
    end
  end
  out
end
