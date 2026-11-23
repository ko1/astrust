# saruby prelude: built-ins written in Ruby (loaded at boot).
module Comparable
  def ==(o)
    return true if equal?(o)
    seen = Enumerable.__ivget(Comparable, :__eqrec) || Enumerable.__ivset(Comparable, :__eqrec, {})
    key = [__id__, o.__id__]
    return false if seen.key?(key)
    seen[key] = true
    begin
      r = (self <=> o)
    ensure
      seen.delete(key)
    end
    r.nil? ? false : r == 0
  end
end

module Enumerable
  # Enumerable.__each(self) { |v| } packs a multi-value yield into one element (CRuby's ENUM_WANT_SVALUE)
  # without Ruby-level calls; methods that hand raw values to the block use each { |*x| yield(*x) }.
  def __size = (respond_to?(:size) ? size : nil)
  private :__size
  def grep(pat)
    a = []
    if block_given?
      Enumerable.__each(self) { |v| a << yield(v) if pat === v }
    else
      # without a block the caller's $~ stays (CRuby's rb_reg_match_p); this frame shares it (svar.rs)
      lm = $~
      Enumerable.__each(self) { |v| a << v if pat === v }
      $~ = lm
    end
    a
  end
  def grep_v(pat)
    a = []
    if block_given?
      Enumerable.__each(self) { |v| a << yield(v) unless pat === v }
    else
      lm = $~
      Enumerable.__each(self) { |v| a << v unless pat === v }
      $~ = lm
    end
    a
  end
  def find_index(*args)
    Enumerable.__argc(args.size, 1, 1) if args.size > 1
    if args.empty?
      return enum_for(:find_index) unless block_given?
      i = 0
      each { |*x| return i if yield(*x); i += 1 }
    else
      warn("warning: given block not used", uplevel: 1) if block_given? && false
      i = 0
      Enumerable.__each(self) { |v| return i if v == args[0]; i += 1 }
    end
    nil
  end
  def each_entry(*args)
    return enum_for(:each_entry, *args) { __size } unless block_given?
    Enumerable.__each(self, *args) { |v| yield v }
    self
  end
  def sum(init = 0, &b)
    return Enumerable.__range_sum(self, init) if !b && Range === self
    a = []
    if b
      Enumerable.__each(self) { |v| a << b.call(v) }
    else
      Enumerable.__each(self) { |v| a << v }
    end
    a.sum(init)
  end
  def first(*args)
    if args.empty?
      Enumerable.__each(self) { |v| return v }
      return nil
    end
    Enumerable.__argc(args.size, 0, 1) if args.size > 1
    n = Enumerable.__int(args[0])
    raise ArgumentError, "attempt to take negative size" if n < 0
    a = []
    return a if n == 0
    Enumerable.__each(self) { |v| a << v; break if a.size == n }
    a
  end
  def take(n)
    n = Enumerable.__int(n)
    raise ArgumentError, "attempt to take negative size" if n < 0
    a = []
    return a if n == 0
    Enumerable.__each(self) { |v| a << v; break if a.size == n }
    a
  end
  def take_while
    return enum_for(:take_while) unless block_given?
    a = []
    each { |*x| break unless yield(*x); a << (x.size <= 1 ? x[0] : x) }
    a
  end
  def drop(n)
    n = Enumerable.__int(n)
    raise ArgumentError, "attempt to drop negative size" if n < 0
    a = []; i = 0
    Enumerable.__each(self) { |v| a << v if i >= n; i += 1 }
    a
  end
  def drop_while
    return enum_for(:drop_while) unless block_given?
    a = []; dropping = true
    Enumerable.__each(self) { |v| dropping = false if dropping && !yield(v); a << v unless dropping }
    a
  end
  def sort(&b) = Enumerable.__to_a(self).sort(&b)
  def sort_by(&b)
    return enum_for(:sort_by) { __size } unless b
    Enumerable.__sort_by(self, &b)
  end
  def chunk_while(&b)
    raise ArgumentError, "tried to create Proc object without a block" unless b
    src = self
    Enumerator.new do |y|
      cur = nil
      Enumerable.__each(src) do |v|
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
    return enum_for(:chunk) { __size } unless b
    src = self
    Enumerator.new do |y|
      ck = nil; cur = nil
      Enumerable.__each(src) do |v|
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
      Enumerable.__argc(pat.size, 1, 1) if pat.size != 1
    end
    pat = pat[0]
    src = self
    Enumerator.new do |y|
      cur = nil
      Enumerable.__each(src) do |v|
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
      Enumerable.__argc(pat.size, 1, 1) if pat.size != 1
    end
    pat = pat[0]
    src = self
    Enumerator.new do |y|
      cur = nil
      Enumerable.__each(src) do |v|
        (cur ||= []) << v
        if b ? b.call(v) : (pat === v) then y << cur; cur = nil end
      end
      y << cur if cur
    end
  end
  def all?(*pat)
    Enumerable.__argc(pat.size, 0, 1) if pat.size > 1
    if !pat.empty? then Enumerable.__each(self) { |v| return false unless pat[0] === v }
    elsif block_given? then each { |*x| return false unless yield(*x) }
    else Enumerable.__each(self) { |v| return false unless v } end
    true
  end
  def any?(*pat)
    Enumerable.__argc(pat.size, 0, 1) if pat.size > 1
    if !pat.empty? then Enumerable.__each(self) { |v| return true if pat[0] === v }
    elsif block_given? then each { |*x| return true if yield(*x) }
    else Enumerable.__each(self) { |v| return true if v } end
    false
  end
  def none?(*pat)
    Enumerable.__argc(pat.size, 0, 1) if pat.size > 1
    if !pat.empty? then Enumerable.__each(self) { |v| return false if pat[0] === v }
    elsif block_given? then each { |*x| return false if yield(*x) }
    else Enumerable.__each(self) { |v| return false if v } end
    true
  end
  def one?(*pat)
    Enumerable.__argc(pat.size, 0, 1) if pat.size > 1
    n = 0
    if !pat.empty? then Enumerable.__each(self) { |v| n += 1 if pat[0] === v; return false if n > 1 }
    elsif block_given? then each { |*x| n += 1 if yield(*x); return false if n > 1 }
    else Enumerable.__each(self) { |v| n += 1 if v; return false if n > 1 } end
    n == 1
  end
  def zip(*others, &b)
    a = to_a
    others = others.map do |o|
      if (oa = Array.try_convert(o)) then oa
      elsif o.respond_to?(:each) then o.respond_to?(:first) ? o.first(a.size) : o.to_enum(:each).first(a.size)
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
      Enumerable.__each(self) { |v| h[v] = v unless h.key?(v) }
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
  def to_set(*args, &b) = Set.new(self, *args, &b)
  def reverse_each(*args, &b)
    return enum_for(:reverse_each, *args) { __size } unless b
    to_a(*args).reverse_each(&b)
    self
  end
  def cycle(*args)
    Enumerable.__argc(args.size, 0, 1) if args.size > 1
    n = args[0]
    unless block_given?
      return enum_for(:cycle, *args) do
        if n.nil? then (s = __size) == 0 ? 0 : Float::INFINITY
        else s = __size; k = Enumerable.__int(n); s && (k < 0 ? 0 : s * k) end
      end
    end
    unless n.nil?
      n = Enumerable.__int(n)
      return nil if n <= 0
    end
    saved = []
    Enumerable.__each(self) { |v| saved << v; yield v }
    return nil if saved.empty?
    if n.nil?
      loop { saved.each { |v| yield v } }
    else
      (n - 1).times { saved.each { |v| yield v } }
    end
    nil
  end
  def chain(*enums) = Enumerable.__chain([self, *enums])
  def compact
    a = []
    Enumerable.__each(self) { |v| a << v unless v.nil? }
    a
  end
end


module GC
  # CRuby's gc.rb signature: keywords, no Hash allocated per call
  def self.start(full_mark: true, immediate_mark: true, immediate_sweep: true) = GC.__start(full_mark)
end

module Kernel
  def frozen_string? = false
  def then_if = self
  def respond_to_all?(*ms) = ms.all? { |m| respond_to?(m) }
  def pp_inspect = inspect
  def instance_variable_names = instance_variables.map(&:to_s)
  def require_optional(f) = (require f rescue nil)
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
  def to_s = ""
  def inspect = "nil"
  def &(o) = false
  def |(o) = !!o
  def ^(o) = !!o
  def nil? = true
  def to_i = 0
  def to_f = 0.0
  def =~(o) = nil
end
class TrueClass
  def to_s = "true"
  alias inspect to_s
  def &(o) = !!o
  def |(o) = true
  def ^(o) = !o
  def ===(o) = equal?(o)
end
class FalseClass
  def to_s = "false"
  alias inspect to_s
  def &(o) = false
  def |(o) = !!o
  def ^(o) = !!o
  def ===(o) = equal?(o)
end

class Integer
  def numerator = self
  def denominator = 1
end

class Struct
end

class Exception
  def ===(o) = super
end

class Enumerator
  def +(other) = Enumerable.__chain([self, other])
  def next_values
    v = self.next
    [v]
  end
  def peek_values = [peek]
  def self.produce(*args, **kw, &b)
    raise ArgumentError, "no block given" unless b
    # RB_SCAN_ARGS_LAST_HASH_KEYWORDS: a trailing Hash is the keywords
    kw = args.pop if kw.empty? && Hash === args.last
    Enumerable.__argc(args.size, 0, 1) if args.size > 1
    Enumerable.__kw(kw, :size) unless kw.empty?
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
      Enumerable.__argc(args.size, 0, 1) if args.size > 1
      unless args.empty?
        raise TypeError, "wrong argument type #{args[0].class} (expected Proc)" unless args[0].is_a?(Proc)
        b = args[0]
      end
      raise LocalJumpError, "no block given" unless b
      @__b = b
    end
    def each(*args, &blk) = @__b.call(Enumerator::Yielder.new(&blk), *args)
  end

  class Product < Enumerator
    def initialize(*enums)
      @__enums = enums
    end
    def each(&b)
      return self unless b
      __each_rec(0, [], &b)
      self
    end
    def __each_rec(i, acc, &b)
      if i == @__enums.size
        b.call(acc.dup)
      else
        @__enums[i].each_entry { |v| acc.push(v); __each_rec(i + 1, acc, &b); acc.pop }
      end
    end
    private :__each_rec
    def size
      @__enums.inject(1) do |t, e|
        return nil unless e.respond_to?(:size)
        s = e.size
        return s if s.nil?
        t * s
      end
    end
    def rewind = (@__enums.reverse_each { |e| e.rewind if e.respond_to?(:rewind) }; self)
    def inspect = @__enums ? "#<Enumerator::Product: #{@__enums.inspect}>" : "#<#{self.class}: uninitialized>"
    alias to_s inspect
  end

  class Chain < Enumerator
    def initialize(*enums)
      @__enums = enums.freeze
    end
    def each(*args, &b)
      return self unless b
      @__enums.each { |e| e.each(*args, &b) }
      self
    end
    def size
      @__enums.inject(0) do |t, e|
        s = e.size
        return s if s.nil? || (s.is_a?(Float) && s.infinite?)
        t + s
      end
    end
    def rewind = (@__enums.reverse_each { |e| e.rewind if e.respond_to?(:rewind) }; self)
    def inspect = @__enums ? "#<Enumerator::Chain: [#{@__enums.map(&:inspect).join(', ')}]>" : "#<#{self.class}: uninitialized>"
    alias to_s inspect
  end
end

class Set
  include Enumerable
  def self.[](*a) = new(a)
  def initialize(enum = nil, &b)
    Enumerable.__ivset(self, :__h, {})
    return if enum.nil?
    raise ArgumentError, "value must be enumerable" unless enum.respond_to?(:each_entry) || enum.respond_to?(:each)
    Enumerable.__set_init(__hash, enum, &b)
  end
  def initialize_copy(o)
    super
    Enumerable.__ivset(self, :__h, o.__hash.dup)
  end
  def __enum(o)
    return o if o.is_a?(Set)
    raise ArgumentError, "value must be enumerable" unless o.respond_to?(:each_entry) || o.respond_to?(:each)
    o
  end
  def __set(o)
    raise ArgumentError, "value must be a set" unless o.is_a?(Set)
    o
  end
  def __new_like = self.class.new
  def __frozen! = (raise FrozenError.new("can't modify frozen #{self.class}: #{inspect}", receiver: self) if frozen?)
  # nil, or true while #each runs (restored on exit: no Integer arithmetic)
  def __iterating? = Enumerable.__ivget(self, :__it)
  private :__enum, :__set, :__new_like, :__frozen!, :__iterating?
  def compare_by_identity
    __frozen!
    __hash.compare_by_identity
    self
  end
  def compare_by_identity? = __hash.compare_by_identity?
  def __hash = Enumerable.__ivget(self, :__h)
  def add(x)
    __frozen!
    h = __hash
    if __iterating? && !h.key?(x)
      raise RuntimeError, "can't add a new item into set during iteration"
    end
    h[x] = true
    self
  end
  alias << add
  def add?(x) = (__hash.key?(x) ? nil : add(x))
  def delete(x)
    __frozen!
    __hash.delete(x)
    self
  end
  def delete?(x) = (__hash.key?(x) ? delete(x) : nil)
  def include?(x) = __hash.key?(x)
  alias member? include?
  alias === include?
  def size = __hash.size
  alias length size
  def empty? = __hash.empty?
  def each(&b)
    return enum_for(:each) { size } unless b
    it = Enumerable.__ivget(self, :__it)
    Enumerable.__ivset(self, :__it, true)
    begin
      __hash.each_key(&b)
    ensure
      Enumerable.__ivset(self, :__it, it)
    end
    self
  end
  def to_a = __hash.keys
  def to_set(*a, &b) = (a.empty? && !b && instance_of?(Set) ? self : Set.new(self, *a, &b))
  def freeze = (__hash.freeze; super)
  def |(o) = (s = dup; s.merge(__enum(o)); s)
  alias union |
  alias + |
  def -(o) = (s = dup; s.subtract(__enum(o)); s)
  alias difference -
  def &(o)
    s = __new_like
    __enum(o).each { |x| s.add(x) if include?(x) }
    s
  end
  alias intersection &
  def ^(o)
    s = dup
    __enum(o).each { |x| s.add(x) unless s.delete?(x) }
    s
  end
  def subset?(o) = (__set(o); size <= o.size && all? { |x| o.include?(x) })
  alias <= subset?
  def superset?(o) = (__set(o); size >= o.size && o.all? { |x| include?(x) })
  alias >= superset?
  def proper_subset?(o) = (__set(o); size < o.size && all? { |x| o.include?(x) })
  alias < proper_subset?
  def proper_superset?(o) = (__set(o); size > o.size && o.all? { |x| include?(x) })
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
    o = __enum(o)
    if o.is_a?(Set) && o.size < size then o.any? { |x| include?(x) }
    elsif o.is_a?(Set) then any? { |x| o.include?(x) }
    else o.any? { |x| include?(x) }
    end
  end
  def merge(*es)
    raise RuntimeError, "cannot add to set during iteration" if __iterating?
    __frozen!
    Enumerable.__set_merge(__hash, es)
    self
  end
  def subtract(e) = (__enum(e).each { |x| delete(x) }; self)
  def replace(e)
    e = __enum(e)
    __frozen!
    raise RuntimeError, "cannot replace set during iteration" if __iterating?
    a = e.to_a
    __hash.clear
    a.each { |x| add(x) }
    self
  end
  def ==(o)
    return true if equal?(o)
    return false unless o.is_a?(Set)
    Enumerable.__set_eq(__hash, o.__hash)
  end
  def hash = to_a.map(&:hash).sort.hash
  def eql?(o) = o.is_a?(Set) && __hash.eql?(o.__hash)
  def clear = (__frozen!; __hash.clear; self)
  def reset = (__frozen!; __hash.rehash; self)
  def map!(&b)
    return enum_for(:map!) { size } unless b
    a = to_a.map(&b)
    __frozen!
    Enumerable.__ivset(self, :__h, {})
    a.each { |x| add(x) }
    self
  end
  alias collect! map!
  def select!(&b)
    return enum_for(:select!) { size } unless b
    n = size; __hash.select! { |k, _| b.call(k) }; n == size ? nil : self
  end
  alias filter! select!
  def reject!(&b)
    return enum_for(:reject!) { size } unless b
    n = size; __hash.reject! { |k, _| b.call(k) }; n == size ? nil : self
  end
  def delete_if(&b) = (return enum_for(:delete_if) { size } unless b; reject!(&b); self)
  def keep_if(&b) = (return enum_for(:keep_if) { size } unless b; select!(&b); self)
  def classify
    return enum_for(:classify) { size } unless block_given?
    h = {}
    each { |x| (h[yield(x)] ||= __new_like).add(x) }
    h
  end
  def divide(&b)
    return enum_for(:divide) { size } unless b
    if b.arity == 2
      items = to_a
      parent = {}
      find = ->(x) { x = parent[x] while parent[x] != x; x }
      items.each { |x| parent[x] = x }
      items.each { |a| items.each { |c| parent[find.(a)] = find.(c) if b.call(a, c) && find.(a) != find.(c) } }
      groups = {}
      items.each { |x| (groups[find.(x)] ||= __new_like).add(x) }
      s = Set.new; groups.each_value { |g| s.add(g) }; s
    else
      s = Set.new; classify(&b).each_value { |g| s.add(g) }; s
    end
  end
  def __flatten_into(out, seen)
    each do |x|
      if x.is_a?(Set)
        raise ArgumentError, "tried to flatten recursive Set" if seen[x.object_id]
        seen[x.object_id] = true
        x.__flatten_into(out, seen)
        seen.delete(x.object_id)
      else
        out.add(x)
      end
    end
    out
  end
  def flatten = __flatten_into(__new_like, { object_id => true })
  def flatten!
    return nil unless any? { |x| x.is_a?(Set) }
    replace(flatten)
  end
  def join(sep = nil) = to_a.join(sep)
  def inspect
    seen = (Enumerable.__ivget(Set, :__insp) || Enumerable.__ivset(Set, :__insp, {}))
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

class Data
  class << self
    undef_method :new
    def define(*syms, &blk)
      syms = syms.map do |s|
        if s.is_a?(Symbol) then s
        elsif s.is_a?(String) then s.to_sym
        else raise TypeError, "#{s.inspect} is not a symbol nor a string"
        end
      end
      seen = {}
      syms.each do |s|
        raise ArgumentError, "invalid data member: #{s}" if s.end_with?("=")
        raise ArgumentError, "duplicate member: #{s}" if seen[s]
        seen[s] = true
      end
      k = Class.new(self)
      Enumerable.__ivset(k, :__members__, syms.freeze)
      syms.each { |m| k.define_method(m) { Enumerable.__ivget(self, m) } }
      class << k
        define_method(:new) do |*args, **kw, &blk|
          mem = __members
          if !args.empty?
            raise ArgumentError, "wrong number of arguments (given #{args.size + 1}, expected 0)" unless kw.empty?
            if args.size > mem.size
              raise ArgumentError, "wrong number of arguments (given #{args.size}, expected 0#{mem.empty? ? '' : "..#{mem.size}"})"
            end
            args.each_with_index { |v, i| kw[mem[i]] = v }
          end
          o = allocate
          o.__send__(:initialize, **kw, &blk)
          o
        end
        alias_method :[], :new
        def __members
          k = self
          k = k.superclass while Enumerable.__ivget(k, :__members__).nil?
          Enumerable.__ivget(k, :__members__)
        end
        def members = __members.dup
      end
      k.class_eval(&blk) if blk
      k
    end
  end
  def initialize(**kw)
    mem = self.class.__members
    missing = mem.reject { |m| kw.key?(m) }
    unknown = kw.keys.reject { |m| mem.include?(m.is_a?(String) ? m.to_sym : m) }
    unless unknown.empty?
      raise ArgumentError, "unknown keyword#{unknown.size > 1 ? 's' : ''}: #{unknown.map(&:inspect).join(', ')}"
    end
    unless missing.empty?
      raise ArgumentError, "missing keyword#{missing.size > 1 ? 's' : ''}: #{missing.map(&:inspect).join(', ')}"
    end
    kw.each { |k, v| Enumerable.__ivset(self, k.to_sym, v) }
    freeze
  end
  private :initialize
  def initialize_copy(o)
    o.class.members.each { |m| Enumerable.__ivset(self, m, Enumerable.__ivget(o, m)) }
    freeze
  end
  private :initialize_copy
  def members = self.class.members
  def to_h(&b)
    h = {}
    members.each do |m|
      v = Enumerable.__ivget(self, m)
      if b
        pr = b.call(m, v)
        ar = Array.try_convert(pr)
        raise TypeError, "wrong element type #{pr.class} (expected array)" unless ar
        raise ArgumentError, "element has wrong array length (expected 2, was #{ar.size})" if ar.size != 2
        h[ar[0]] = ar[1]
      else
        h[m] = v
      end
    end
    h
  end
  def deconstruct = members.map { |m| Enumerable.__ivget(self, m) }
  def deconstruct_keys(keys)
    return to_h if keys.nil?
    raise TypeError, "wrong argument type #{keys.class} (expected Array or nil)" unless keys.is_a?(Array)
    h = {}
    mem = members
    return h if keys.size > mem.size
    keys.each do |k|
      s = k.is_a?(String) ? k.to_sym : k
      break unless s.is_a?(Symbol) && mem.include?(s)
      h[k] = Enumerable.__ivget(self, s)
    end
    h
  end
  def with(*args, **kw)
    raise ArgumentError, "wrong number of arguments (given #{args.size}, expected 0)" unless args.empty?
    return self if kw.empty?
    self.class.new(**to_h.merge(kw))
  end
  def ==(o)
    return true if equal?(o)
    return false unless o.class == self.class
    members.all? { |m| Enumerable.__ivget(self, m) == Enumerable.__ivget(o, m) }
  end
  def eql?(o)
    return true if equal?(o)
    return false unless o.class == self.class
    members.all? { |m| Enumerable.__ivget(self, m).eql?(Enumerable.__ivget(o, m)) }
  end
  def hash = [self.class, *deconstruct].hash
  def inspect
    # rb_class_path, not #name; a path under an anonymous module is left out
    name = Module.instance_method(:name).bind_call(self.class)
    name = nil if name&.start_with?("#")
    seen = (Enumerable.__ivget(Data, :__insp) || Enumerable.__ivset(Data, :__insp, {}))
    return "#<data #{name}:...>" if seen.key?(object_id)
    seen[object_id] = true
    begin
      parts = members.map do |m|
        v = Enumerable.__ivget(self, m)
        "#{m}=#{v.inspect}"
      end
      s = "#<data "
      s << name << (parts.empty? ? "" : " ") if name
      s << parts.join(", ") << ">"
    ensure
      seen.delete(object_id)
    end
  end
  alias to_s inspect
end

class Range
  # range.c: an endless range has no set (Enumerable#to_set would never end)
  def to_set(*args, &b)
    raise RangeError, "cannot convert endless range to a set" if self.end.nil?
    super
  end
end
