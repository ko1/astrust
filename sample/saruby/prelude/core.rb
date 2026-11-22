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
  def >(o)  = __cmp_or_fail(o) > 0
  def >=(o) = __cmp_or_fail(o) >= 0
  def <(o)  = __cmp_or_fail(o) < 0
  def <=(o) = __cmp_or_fail(o) <= 0
  def between?(a, b) = self >= a && self <= b
  def clamp(a, b = nil)
    if b.nil? && a.is_a?(Range)
      lo, hi = a.begin, a.end
      raise ArgumentError, "cannot clamp with an exclusive range" if a.exclude_end? && !hi.nil?
    else
      lo, hi = a, b
    end
    raise ArgumentError, "min argument must be less than or equal to max argument" if !lo.nil? && !hi.nil? && (lo <=> hi) > 0
    return lo if !lo.nil? && (self <=> lo) < 0
    return hi if !hi.nil? && (self <=> hi) > 0
    self
  end
  private def __cmp_or_fail(o)
    r = (self <=> o)
    if r.nil?
      on = o.nil? || o == true || o == false || o.is_a?(Numeric) ? o.inspect : o.class
      raise ArgumentError, "comparison of #{self.class} with #{on} failed"
    end
    r
  end
end

module Enumerable
  # `each { |*x| v = x.size <= 1 ? x[0] : x }` packs a multi-value yield into one element
  # (CRuby's ENUM_WANT_SVALUE); methods that hand raw values to the block use yield(*x).
  def __size = (respond_to?(:size) ? size : nil)
  private :__size
  def grep(pat)
    a = []
    if block_given?
      each { |*x| v = x.size <= 1 ? x[0] : x; a << yield(v) if pat === v }
    else
      each { |*x| v = x.size <= 1 ? x[0] : x; a << v if pat === v }
    end
    a
  end
  def grep_v(pat)
    a = []
    if block_given?
      each { |*x| v = x.size <= 1 ? x[0] : x; a << yield(v) unless pat === v }
    else
      each { |*x| v = x.size <= 1 ? x[0] : x; a << v unless pat === v }
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
      each { |*x| v = x.size <= 1 ? x[0] : x; return i if v == args[0]; i += 1 }
    end
    nil
  end
  def each_entry(*args)
    return enum_for(:each_entry, *args) { __size } unless block_given?
    each(*args) { |*x| yield(x.size <= 1 ? x[0] : x) }
    self
  end
  def sum(init = 0, &b)
    return Enumerable.__range_sum(self, init) if !b && Range === self
    a = []
    if b
      each { |*x| a << b.call(x.size <= 1 ? x[0] : x) }
    else
      each { |*x| a << (x.size <= 1 ? x[0] : x) }
    end
    a.sum(init)
  end
  def first(*args)
    if args.empty?
      each { |*x| return x.size <= 1 ? x[0] : x }
      return nil
    end
    Enumerable.__argc(args.size, 0, 1) if args.size > 1
    n = Enumerable.__int(args[0])
    raise ArgumentError, "attempt to take negative size" if n < 0
    a = []
    return a if n == 0
    each { |*x| a << (x.size <= 1 ? x[0] : x); break if a.size >= n }
    a
  end
  def take(n)
    n = Enumerable.__int(n)
    raise ArgumentError, "attempt to take negative size" if n < 0
    a = []
    return a if n == 0
    each { |*x| a << (x.size <= 1 ? x[0] : x); break if a.size >= n }
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
    each { |*x| a << (x.size <= 1 ? x[0] : x) if i >= n; i += 1 }
    a
  end
  def drop_while
    return enum_for(:drop_while) unless block_given?
    a = []; dropping = true
    each { |*x| v = x.size <= 1 ? x[0] : x; dropping = false if dropping && !yield(v); a << v unless dropping }
    a
  end
  def min(n = nil, &b) = to_a.min(n, &b)
  def max(n = nil, &b) = to_a.max(n, &b)
  def minmax(&b) = to_a.minmax(&b)
  def sort(&b) = to_a.sort(&b)
  def sort_by(&b)
    return enum_for(:sort_by) { __size } unless b
    to_a.sort_by(&b)
  end
  def min_by(n = nil, &b)
    return enum_for(:min_by, *n) { __size } unless b
    return sort_by(&b).first(Enumerable.__int(n)) if n
    best = nil; bk = nil; first = true
    each { |*x| v = x.size <= 1 ? x[0] : x; k = b.call(v); if first || Enumerable.__cmp(k, bk) < 0 then best = v; bk = k; first = false end }
    best
  end
  def max_by(n = nil, &b)
    return enum_for(:max_by, *n) { __size } unless b
    return sort_by(&b).reverse.first(Enumerable.__int(n)) if n
    best = nil; bk = nil; first = true
    each { |*x| v = x.size <= 1 ? x[0] : x; k = b.call(v); if first || Enumerable.__cmp(k, bk) > 0 then best = v; bk = k; first = false end }
    best
  end
  def minmax_by(&b)
    return enum_for(:minmax_by) { __size } unless b
    mn = mx = nil; mnk = mxk = nil; first = true
    each do |*x|
      v = x.size <= 1 ? x[0] : x; k = b.call(v)
      if first then mn = mx = v; mnk = mxk = k; first = false
      else
        if Enumerable.__cmp(k, mnk) < 0 then mn = v; mnk = k end
        if Enumerable.__cmp(k, mxk) > 0 then mx = v; mxk = k end
      end
    end
    [mn, mx]
  end
  def partition
    return enum_for(:partition) { __size } unless block_given?
    t = []; f = []
    each { |*x| v = x.size <= 1 ? x[0] : x; (yield(v) ? t : f) << v }
    [t, f]
  end
  def chunk_while(&b)
    raise ArgumentError, "tried to create Proc object without a block" unless b
    src = self
    Enumerator.new do |y|
      cur = nil
      src.each do |*x|
        v = x.size <= 1 ? x[0] : x
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
      src.each do |*x|
        v = x.size <= 1 ? x[0] : x
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
      src.each do |*x|
        v = x.size <= 1 ? x[0] : x
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
      src.each do |*x|
        v = x.size <= 1 ? x[0] : x
        (cur ||= []) << v
        if b ? b.call(v) : (pat === v) then y << cur; cur = nil end
      end
      y << cur if cur
    end
  end
  def tally(h = nil)
    if nil.equal?(h)
      h = {}
    else
      h = Enumerable.__hash(h)
      raise FrozenError.new("can't modify frozen #{h.class}: #{h.inspect}", receiver: h) if h.frozen?
    end
    each do |*x|
      v = x.size <= 1 ? x[0] : x
      n = h.key?(v) ? h[v] : 0
      raise TypeError, "wrong argument type #{n.class} (expected Integer)" unless n.is_a?(Integer)
      h[v] = n + 1
    end
    h
  end
  def all?(*pat)
    Enumerable.__argc(pat.size, 0, 1) if pat.size > 1
    if !pat.empty? then each { |*x| v = x.size <= 1 ? x[0] : x; return false unless pat[0] === v }
    elsif block_given? then each { |*x| return false unless yield(*x) }
    else each { |*x| v = x.size <= 1 ? x[0] : x; return false unless v } end
    true
  end
  def any?(*pat)
    Enumerable.__argc(pat.size, 0, 1) if pat.size > 1
    if !pat.empty? then each { |*x| v = x.size <= 1 ? x[0] : x; return true if pat[0] === v }
    elsif block_given? then each { |*x| return true if yield(*x) }
    else each { |*x| v = x.size <= 1 ? x[0] : x; return true if v } end
    false
  end
  def none?(*pat)
    Enumerable.__argc(pat.size, 0, 1) if pat.size > 1
    if !pat.empty? then each { |*x| v = x.size <= 1 ? x[0] : x; return false if pat[0] === v }
    elsif block_given? then each { |*x| return false if yield(*x) }
    else each { |*x| v = x.size <= 1 ? x[0] : x; return false if v } end
    true
  end
  def one?(*pat)
    Enumerable.__argc(pat.size, 0, 1) if pat.size > 1
    n = 0
    if !pat.empty? then each { |*x| v = x.size <= 1 ? x[0] : x; n += 1 if pat[0] === v; return false if n > 1 }
    elsif block_given? then each { |*x| n += 1 if yield(*x); return false if n > 1 }
    else each { |*x| v = x.size <= 1 ? x[0] : x; n += 1 if v; return false if n > 1 } end
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
      each { |*x| v = x.size <= 1 ? x[0] : x; h[v] = v unless h.key?(v) }
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
  def cycle(n = nil)
    unless block_given?
      return enum_for(:cycle, n) do
        if n.nil? then (s = __size) == 0 ? 0 : Float::INFINITY
        else s = __size; k = Enumerable.__int(n); s && (k < 0 ? 0 : s * k) end
      end
    end
    unless n.nil?
      n = Enumerable.__int(n)
      return nil if n <= 0
    end
    saved = []
    each { |*x| v = x.size <= 1 ? x[0] : x; saved << v; yield v }
    return nil if saved.empty?
    if n.nil?
      loop { saved.each { |v| yield v } }
    else
      (n - 1).times { saved.each { |v| yield v } }
    end
    nil
  end
  def chain(*enums) = Enumerator::Chain.new(self, *enums)
  def lazy = Enumerator::Lazy.__from(self, :each, [], :__src_size)
  def compact
    a = []
    each { |*x| v = x.size <= 1 ? x[0] : x; a << v unless v.nil? }
    a
  end
end


class Object
  def instance_variables_hash = nil
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
  def +(other) = Enumerator::Chain.new(self, other)
  def next_values
    v = self.next
    [v]
  end
  def peek_values = [peek]
  def self.produce(*args, &b)
    raise ArgumentError, "no block given" unless b
    Enumerable.__argc(args.size, 0, 1) if args.size > 1
    Enumerator.new(Float::INFINITY) do |y|
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
    def +(other) = Chain.new(self, other)
    def rewind = (@__enums.reverse_each { |e| e.rewind if e.respond_to?(:rewind) }; self)
    def inspect = @__enums ? "#<Enumerator::Chain: [#{@__enums.map(&:inspect).join(', ')}]>" : "#<#{self.class}: uninitialized>"
    alias to_s inspect
  end

  class Lazy < Enumerator
    # A Lazy is a source (@__src, iterated with @__meth/@__margs) or a parent Lazy plus one stage (@__op).
    def initialize(obj, size = nil, &blk)
      raise ArgumentError, "tried to call lazy new without a block" unless blk
      @__src = obj; @__size = size; @__gen = blk; @__meth = :each; @__margs = []
      @__parent = nil; @__op = nil; @__mid = :each; @__args = nil
    end
    def self.__from(src, meth = :each, margs = [], size = nil)
      l = allocate
      l.__init_src(src, meth, margs, size)
      l
    end
    def __init_src(src, meth, margs, size)
      @__src = src; @__meth = meth; @__margs = margs; @__size = size; @__gen = nil
      @__parent = nil; @__op = nil; @__mid = nil; @__args = nil
    end
    def __init_op(parent, op, mid, args)
      @__parent = parent; @__op = op; @__mid = mid; @__args = args
    end
    def __add(mid, args, *op)
      l = Lazy.allocate
      l.__init_op(self, op, mid, args)
      l
    end
    def __stages
      st = []; l = self
      while l.instance_variable_get(:@__parent)
        st.unshift(l.instance_variable_get(:@__op))
        l = l.instance_variable_get(:@__parent)
      end
      [l, st]
    end
    def __src_each(&sink)
      if @__gen
        y = Enumerator::Yielder.new { |*v| sink.call(v.size <= 1 ? v[0] : v) }
        @__src.each { |*x| @__gen.call(y, *x) }
      else
        @__src.__send__(@__meth, *@__margs) { |*x| sink.call(x.size <= 1 ? x[0] : x) }
      end
    end

    def each(*args, &blk)
      unless args.empty?
        l = @__parent ? self : Lazy.__from(@__src, @__meth, @__margs + args, @__size)
        l = Lazy.__from(self, :each, args) if @__parent || @__gen
        return blk ? l.each(&blk) : l
      end
      return self unless blk
      return @__src.__send__(@__meth, *@__margs, &blk) if Enumerable.__ivget(self, :__blkmeth)
      root, ops = __stages
      return self if ops.any? { |o| o[0] == :take && o[1] == 0 }
      tag = Object.new
      sink = blk
      ops.reverse_each { |op| sink = Lazy.__stage(op, sink, tag) }
      catch(tag) { root.__src_each(&sink) }
      self
    end

    # a packed multi-value element is spread for a lambda that takes several (rb_yield_force_blockarg)
    def self.__fc(f, x) = (x.is_a?(Array) && f.lambda? && f.arity != 1 && f.arity != -1 ? f.call(*x) : f.call(x))
    def self.__stage(op, nxt, tag)
      kind, f, g = op
      case kind
      when :map then ->(x) { nxt.call(Lazy.__fc(f, x)) }
      when :select then ->(x) { nxt.call(x) if Lazy.__fc(f, x) }
      when :reject then ->(x) { nxt.call(x) unless Lazy.__fc(f, x) }
      when :grep then ->(x) { nxt.call(g ? Lazy.__fc(g, x) : x) if f === x }
      when :grep_v then ->(x) { nxt.call(g ? Lazy.__fc(g, x) : x) unless f === x }
      when :filter_map then ->(x) { r = Lazy.__fc(f, x); nxt.call(r) if r }
      when :compact then ->(x) { nxt.call(x) unless x.nil? }
      when :flat_map
        lambda do |x|
          r = Lazy.__fc(f, x)
          if (ra = Array.try_convert(r)) then ra.each { |y| nxt.call(y) }
          elsif r.respond_to?(:force) && r.respond_to?(:each) then r.each { |y| nxt.call(y) }
          else nxt.call(r)
          end
        end
      when :take_while then ->(x) { Lazy.__fc(f, x) ? nxt.call(x) : throw(tag) }
      when :drop_while
        dropping = true
        ->(x) { dropping = false if dropping && !Lazy.__fc(f, x); nxt.call(x) unless dropping }
      when :take
        n = 0
        ->(x) { n += 1; nxt.call(x); throw(tag) if n >= f }
      when :drop
        n = 0
        ->(x) { n += 1; nxt.call(x) if n > f }
      when :with_index
        i = f
        if g then ->(x) { g.call(x, i); i += 1; nxt.call(x) }
        else ->(x) { j = i; i += 1; nxt.call([x, j]) }
        end
      when :uniq
        seen = {}
        ->(x) { k = f ? Lazy.__fc(f, x) : x; unless seen.key?(k) then seen[k] = true; nxt.call(x) end }
      when :zip
        i = 0
        ->(x) { row = [x] + f.map { |o| o[i] }; i += 1; nxt.call(row) }
      end
    end

    def __args_s = (@__args.nil? || @__args.empty? ? "" : "(#{@__args.map(&:inspect).join(', ')})")
    def inspect
      if @__parent
        "#<#{self.class}: #{@__parent.inspect}:#{@__mid}#{__args_s}>"
      elsif @__margs.nil? && !@__gen
        "#<#{self.class}: uninitialized>"
      elsif @__gen
        "#<#{self.class}: #{@__src.inspect}:each>"
      elsif @__meth == :each && @__margs.empty?
        "#<#{self.class}: #{@__src.inspect}>"
      else
        "#<#{self.class}: #{@__src.inspect}:#{@__meth}#{@__margs.empty? ? '' : "(#{@__margs.map(&:inspect).join(', ')})"}>"
      end
    end
    alias to_s inspect
    private :__args_s

    def size
      if @__parent
        ps = @__parent.size
        case @__op[0]
        when :map, :with_index, :zip then ps
        when :take then ps.nil? ? nil : [ps, @__op[1]].min
        when :drop then ps.nil? ? nil : [ps - @__op[1], 0].max
        else nil
        end
      else
        s = @__size
        if s.equal?(:__src_size) then @__src.respond_to?(:size) ? @__src.size : nil
        elsif s.respond_to?(:call) then s.call
        else s
        end
      end
    end

    def map(&f)
      raise ArgumentError, "tried to call lazy map without a block" unless f
      __add(:map, nil, :map, f)
    end
    alias collect map
    def select(&f)
      raise ArgumentError, "tried to call lazy select without a block" unless f
      __add(:select, nil, :select, f)
    end
    alias filter select
    alias find_all select
    def reject(&f)
      raise ArgumentError, "tried to call lazy reject without a block" unless f
      __add(:reject, nil, :reject, f)
    end
    def grep(pat, &f) = __add(:grep, [pat], :grep, pat, f)
    def grep_v(pat, &f) = __add(:grep_v, [pat], :grep_v, pat, f)
    def filter_map(&f)
      raise ArgumentError, "tried to call lazy filter_map without a block" unless f
      __add(:filter_map, nil, :filter_map, f)
    end
    def flat_map(&f)
      raise ArgumentError, "tried to call lazy flat_map without a block" unless f
      __add(:flat_map, nil, :flat_map, f)
    end
    alias collect_concat flat_map
    def take_while(&f)
      raise ArgumentError, "tried to call lazy take_while without a block" unless f
      __add(:take_while, nil, :take_while, f)
    end
    def drop_while(&f)
      raise ArgumentError, "tried to call lazy drop_while without a block" unless f
      __add(:drop_while, nil, :drop_while, f)
    end
    def take(n)
      n = Enumerable.__int(n)
      raise ArgumentError, "attempt to take negative size" if n < 0
      __add(:take, [n], :take, n)
    end
    def drop(n)
      n = Enumerable.__int(n)
      raise ArgumentError, "attempt to drop negative size" if n < 0
      __add(:drop, [n], :drop, n)
    end
    def with_index(off = 0, &f)
      off = off.nil? ? 0 : Enumerable.__int(off)
      __add(:with_index, [off], :with_index, off, f)
    end
    def each_with_index(&f) = __add(:each_with_index, nil, :with_index, 0, f)
    def compact = __add(:compact, nil, :compact)
    def uniq(&f) = __add(:uniq, nil, :uniq, f)
    def zip(*others, &b)
      return super if b
      arrs = others.map do |o|
        if (oa = Array.try_convert(o)) then oa
        elsif o.respond_to?(:each) then o
        else raise TypeError, "wrong argument type #{o.class} (must respond to :each)"
        end
      end
      if arrs.all? { |o| o.is_a?(Array) }
        __add(:zip, others, :zip, arrs)
      else
        __add(:zip, others, :zip, arrs.map { |o| o.is_a?(Array) ? o : __zip_cache(o) })
      end
    end
    def __zip_cache(o)
      c = Object.new
      c.instance_variable_set(:@e, o.each_entry.lazy)
      c.instance_variable_set(:@buf, [])
      def c.[](i)
        while @buf.size <= i
          more = @e.first(@buf.size + 1)
          return nil if more.size <= @buf.size
          @buf = more
        end
        @buf[i]
      end
      c
    end
    private :__zip_cache
    def chunk(*a, &b)
      return super.lazy if b
      l = Lazy.__from(self, :chunk, a, :__src_size)
      Enumerable.__ivset(l, :__blkmeth, true)
      l
    end
    def chunk_while(*a, &b) = super.lazy
    def slice_after(*a, &b) = super.lazy
    def slice_before(*a, &b) = super.lazy
    def slice_when(*a, &b) = super.lazy
    def to_enum(meth = :each, *args, &sz) = Lazy.__from(self, meth, args, sz)
    alias enum_for to_enum
    def lazy = self
    def eager
      lz = self
      Kernel.instance_method(:to_enum).bind_call(self, :each) { lz.size }
    end
    def to_a = (a = []; each { |x| a << x }; a)
    alias force to_a
    def first(*args)
      if args.empty?
        each { |x| return x }
        return nil
      end
      super
    end
    def +(other) = Chain.new(self, other).lazy
  end
end

class Set
  include Enumerable
  def self.[](*a) = new(a)
  def initialize(enum = nil, &b)
    Enumerable.__ivset(self, :__h, {})
    return if enum.nil?
    raise ArgumentError, "value must be enumerable" unless enum.respond_to?(:each_entry) || enum.respond_to?(:each)
    if b
      enum.each { |*x| add(b.call(x.size <= 1 ? x[0] : x)) }
    else
      enum.each { |*x| add(x.size <= 1 ? x[0] : x) }
    end
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
  def __iterating? = Enumerable.__ivget(self, :__it).to_i > 0
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
    Enumerable.__ivset(self, :__it, Enumerable.__ivget(self, :__it).to_i + 1)
    begin
      __hash.each_key(&b)
    ensure
      Enumerable.__ivset(self, :__it, Enumerable.__ivget(self, :__it) - 1)
    end
    self
  end
  def to_a = __hash.keys
  def to_set(*a, &b) = (a.empty? && !b && instance_of?(Set) ? self : Set.new(self, *a, &b))
  def freeze = (super)
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
    es.each { |e| __enum(e).each { |x| add(x) } }
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
    return false unless o.is_a?(Set) && size == o.size
    return false if compare_by_identity? != o.compare_by_identity?
    oh = o.__hash
    __hash.each_key { |x| return false unless oh.key?(x) }
    true
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
          mem = members
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
        def members
          k = self
          k = k.superclass while Enumerable.__ivget(k, :__members__).nil?
          Enumerable.__ivget(k, :__members__)
        end
      end
      k.class_eval(&blk) if blk
      k
    end
  end
  def initialize(**kw)
    mem = self.class.members
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
    name = self.class.name
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
