# koruby prelude: built-ins written in Ruby (loaded at boot).
module Comparable
  def ==(o)
    return true if equal?(o)
    r = (self <=> o)
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
  def to_a(*args)
    a = []
    each(*args) { |*x| a << (x.size == 1 ? x[0] : x) }
    a
  end
  alias entries to_a
  def map
    return enum_for(:map) unless block_given?
    a = []
    each { |*x| a << yield(*x) }
    a
  end
  alias collect map
  def flat_map
    return enum_for(:flat_map) unless block_given?
    a = []
    each { |*x| r = yield(*x); r.is_a?(Array) ? a.concat(r) : a << r }
    a
  end
  alias collect_concat flat_map
  def select
    return enum_for(:select) unless block_given?
    a = []
    each { |*x| v = x.size == 1 ? x[0] : x; a << v if yield(*x) }
    a
  end
  alias filter select
  def filter_map
    return enum_for(:filter_map) unless block_given?
    a = []
    each { |*x| r = yield(*x); a << r if r }
    a
  end
  def reject
    return enum_for(:reject) unless block_given?
    a = []
    each { |*x| v = x.size == 1 ? x[0] : x; a << v unless yield(*x) }
    a
  end
  def find(ifnone = nil)
    return enum_for(:find, ifnone) unless block_given?
    each { |*x| v = x.size == 1 ? x[0] : x; return v if yield(*x) }
    ifnone&.call
  end
  alias detect find
  def find_index(*args)
    return enum_for(:find_index) if args.empty? && !block_given?
    i = 0
    if args.empty?
      each { |*x| return i if yield(*x); i += 1 }
    else
      each { |*x| v = x.size == 1 ? x[0] : x; return i if v == args[0]; i += 1 }
    end
    nil
  end
  def each_with_index(*args)
    return enum_for(:each_with_index, *args) unless block_given?
    i = 0
    each(*args) { |*x| yield((x.size == 1 ? x[0] : x), i); i += 1 }
    self
  end
  def each_with_object(memo)
    return enum_for(:each_with_object, memo) unless block_given?
    each { |*x| yield((x.size == 1 ? x[0] : x), memo) }
    memo
  end
  def inject(*args)
    op = nil
    if args.size == 2
      acc, op = args
    elsif args.size == 1 && !block_given?
      op = args[0]; first = true
    elsif args.size == 1
      acc = args[0]
    else
      first = true
    end
    each do |*x|
      v = x.size == 1 ? x[0] : x
      if first
        acc = v; first = false
      else
        acc = op ? acc.__send__(op, v) : yield(acc, v)
      end
    end
    acc
  end
  alias reduce inject
  def sum(init = 0)
    acc = init
    each { |*x| v = x.size == 1 ? x[0] : x; acc += block_given? ? yield(v) : v }
    acc
  end
  def count(*args)
    n = 0
    if !args.empty?
      each { |*x| v = x.size == 1 ? x[0] : x; n += 1 if v == args[0] }
    elsif block_given?
      each { |*x| n += 1 if yield(*x) }
    else
      each { n += 1 }
    end
    n
  end
  def include?(o)
    each { |*x| v = x.size == 1 ? x[0] : x; return true if v == o }
    false
  end
  alias member? include?
  def first(n = nil)
    if n.nil?
      each { |*x| return x.size == 1 ? x[0] : x }
      return nil
    end
    raise ArgumentError, "attempt to take negative size" if n < 0
    a = []
    return a if n == 0
    each { |*x| a << (x.size == 1 ? x[0] : x); break if a.size >= n }
    a
  end
  def take(n)
    raise ArgumentError, "attempt to take negative size" if n < 0
    first(n)
  end
  def take_while
    return enum_for(:take_while) unless block_given?
    a = []
    each { |*x| v = x.size == 1 ? x[0] : x; break unless yield(*x); a << v }
    a
  end
  def drop(n)
    raise ArgumentError, "attempt to drop negative size" if n < 0
    a = []; i = 0
    each { |*x| a << (x.size == 1 ? x[0] : x) if i >= n; i += 1 }
    a
  end
  def drop_while
    return enum_for(:drop_while) unless block_given?
    a = []; dropping = true
    each { |*x| v = x.size == 1 ? x[0] : x; dropping = false if dropping && !yield(*x); a << v unless dropping }
    a
  end
  def min(n = nil, &b) = to_a.min(n, &b)
  def max(n = nil, &b) = to_a.max(n, &b)
  def minmax(&b) = to_a.minmax(&b)
  def sort(&b) = to_a.sort(&b)
  def sort_by(&b)
    return enum_for(:sort_by) unless b
    to_a.sort_by(&b)
  end
  def min_by(n = nil, &b)
    return enum_for(:min_by, n) unless b
    a = to_a
    return a.sort_by(&b).first(n) if n
    best = nil; bk = nil; first = true
    a.each { |v| k = b.call(v); if first || (k <=> bk) < 0 then best = v; bk = k; first = false end }
    best
  end
  def max_by(n = nil, &b)
    return enum_for(:max_by, n) unless b
    a = to_a
    return a.sort_by(&b).reverse.first(n) if n
    best = nil; bk = nil; first = true
    a.each { |v| k = b.call(v); if first || (k <=> bk) > 0 then best = v; bk = k; first = false end }
    best
  end
  def minmax_by(&b)
    return enum_for(:minmax_by) unless b
    [min_by(&b), max_by(&b)]
  end
  def group_by
    return enum_for(:group_by) unless block_given?
    h = {}
    each { |*x| v = x.size == 1 ? x[0] : x; (h[yield(*x)] ||= []) << v }
    h
  end
  def partition
    return enum_for(:partition) unless block_given?
    t = []; f = []
    each { |*x| v = x.size == 1 ? x[0] : x; (yield(*x) ? t : f) << v }
    [t, f]
  end
  def chunk_while(&b)
    res = []
    cur = nil
    each do |*x|
      v = x.size == 1 ? x[0] : x
      if cur.nil? then cur = [v]
      elsif b.call(cur.last, v) then cur << v
      else res << cur; cur = [v] end
    end
    res << cur if cur
    res.each
  end
  def slice_when(&b) = chunk_while { |a, c| !b.call(a, c) }
  def chunk
    return enum_for(:chunk) unless block_given?
    res = []
    each do |*x|
      v = x.size == 1 ? x[0] : x
      k = yield(v)
      next if k.nil? || k == :_separator
      if k == :_alone then res << [k, [v]]
      elsif !res.empty? && res.last[0] == k then res.last[1] << v
      else res << [k, [v]] end
    end
    res.each
  end
  def slice_before(pat = nil, &b)
    res = []
    each do |*x|
      v = x.size == 1 ? x[0] : x
      hit = b ? b.call(v) : (pat === v)
      if res.empty? || hit then res << [v] else res.last << v end
    end
    res.each
  end
  def slice_after(pat = nil, &b)
    res = []; cut = true
    each do |*x|
      v = x.size == 1 ? x[0] : x
      if cut then res << [v] else res.last << v end
      cut = b ? b.call(v) : (pat === v)
    end
    res.each
  end
  def tally(h = {})
    each { |*x| v = x.size == 1 ? x[0] : x; h[v] = (h[v] || 0) + 1 }
    h
  end
  def each_slice(n)
    raise ArgumentError, "invalid slice size" if n <= 0
    return enum_for(:each_slice, n) unless block_given?
    buf = []
    each { |*x| buf << (x.size == 1 ? x[0] : x); if buf.size == n then yield buf; buf = [] end }
    yield buf unless buf.empty?
    self
  end
  def each_cons(n)
    raise ArgumentError, "invalid size" if n <= 0
    return enum_for(:each_cons, n) unless block_given?
    buf = []
    each { |*x| buf << (x.size == 1 ? x[0] : x); buf.shift if buf.size > n; yield buf.dup if buf.size == n }
    self
  end
  def each_entry(*args, &b)
    return enum_for(:each_entry, *args) unless b
    each(*args) { |*x| b.call(x.size == 1 ? x[0] : x) }
    self
  end
  def all?(*pat)
    if !pat.empty? then each { |*x| v = x.size == 1 ? x[0] : x; return false unless pat[0] === v }
    elsif block_given? then each { |*x| return false unless yield(*x) }
    else each { |*x| v = x.size == 1 ? x[0] : x; return false unless v } end
    true
  end
  def any?(*pat)
    if !pat.empty? then each { |*x| v = x.size == 1 ? x[0] : x; return true if pat[0] === v }
    elsif block_given? then each { |*x| return true if yield(*x) }
    else each { |*x| v = x.size == 1 ? x[0] : x; return true if v } end
    false
  end
  def none?(*pat)
    if !pat.empty? then each { |*x| v = x.size == 1 ? x[0] : x; return false if pat[0] === v }
    elsif block_given? then each { |*x| return false if yield(*x) }
    else each { |*x| v = x.size == 1 ? x[0] : x; return false if v } end
    true
  end
  def one?(*pat)
    n = 0
    if !pat.empty? then each { |*x| v = x.size == 1 ? x[0] : x; n += 1 if pat[0] === v; return false if n > 1 }
    elsif block_given? then each { |*x| n += 1 if yield(*x); return false if n > 1 }
    else each { |*x| v = x.size == 1 ? x[0] : x; n += 1 if v; return false if n > 1 } end
    n == 1
  end
  def zip(*others, &b) = to_a.zip(*others, &b)
  def uniq(&b) = to_a.uniq(&b)
  def to_h(*args, &b) = to_a.to_h(&b)
  def to_set = Set.new(self)
  def reverse_each(&b)
    return enum_for(:reverse_each) unless b
    to_a.reverse_each(&b)
    self
  end
  def cycle(n = nil, &b)
    return enum_for(:cycle, n) unless b
    to_a.cycle(n, &b)
  end
  def lazy = Enumerator::Lazy.new(self)
  def compact = to_a.compact
end


class Object
  def instance_variables_hash = nil
end

module Kernel
  def frozen_string? = false
  def then_if = self
  def respond_to_all?(*ms) = ms.all? { |m| respond_to?(m) }
  def Rational(a, b = 1) = Rational.__new(a, b)
  def Complex(a, b = 0) = Complex.__new(a, b)
  def pp_inspect = inspect
  def instance_variable_names = instance_variables.map(&:to_s)
  def open(path, *args, &b) = File.open(path, *args, &b)
  def require_optional(f) = (require f rescue nil)
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
  def to_r = Rational(0, 1)
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
  def to_c = Complex(self, 0)
  def to_r = Rational(self, 1)
  def rationalize(eps = nil) = Rational(self, 1)
  def numerator = self
  def denominator = 1
end
class Float
  def to_r = Rational.__from_float(self)
  def rationalize(eps = nil) = Rational.__from_float(self)
end

class Struct
end

class Exception
  def ===(o) = super
end

class Enumerator
  class Lazy < Enumerator
    def initialize(src, &blk)
      @src = src
      @ops = []
    end
    def __add(op, f) = (l = Lazy.new(@src); l.__ops = @ops + [[op, f]]; l)
    def __ops=(o); @ops = o; end
    def map(&f) = __add(:map, f)
    alias collect map
    def select(&f) = __add(:select, f)
    alias filter select
    def reject(&f) = __add(:reject, f)
    def take_while(&f) = __add(:take_while, f)
    def drop_while(&f) = __add(:drop_while, f)
    def filter_map(&f) = __add(:filter_map, f)
    def flat_map(&f) = __add(:flat_map, f)
    def with_index(off = 0, &f) = __add(:with_index, [off, f])
    def compact = __add(:select, proc { |x| !x.nil? })
    def take(n) = __add(:take, n)
    def drop(n) = __add(:drop, n)
    def uniq = __add(:uniq, nil)
    def lazy = self
    def each(&blk)
      return self unless blk
      counts = Hash.new(0)
      states = {}
      catch(:__lazy_done) do
        @src.each do |*xs|
          x = xs.size == 1 ? xs[0] : xs
          __feed(x, 0, counts, states, blk)
        end
      end
      self
    end
    def __feed(x, i, counts, states, blk)
      if i == @ops.size
        blk.call(x)
        return
      end
      op, f = @ops[i]
      case op
      when :map then __feed(f.call(x), i + 1, counts, states, blk)
      when :select then __feed(x, i + 1, counts, states, blk) if f.call(x)
      when :reject then __feed(x, i + 1, counts, states, blk) unless f.call(x)
      when :filter_map then (r = f.call(x)) && __feed(r, i + 1, counts, states, blk)
      when :flat_map then r = f.call(x); (r.is_a?(Array) ? r : [r]).each { |y| __feed(y, i + 1, counts, states, blk) }
      when :take_while then f.call(x) ? __feed(x, i + 1, counts, states, blk) : throw(:__lazy_done)
      when :drop_while
        states[i] = true unless states[i] || f.call(x)
        __feed(x, i + 1, counts, states, blk) if states[i]
      when :with_index
        off, g = f
        idx = counts[i] + off; counts[i] += 1
        g ? (g.call(x, idx); __feed(x, i + 1, counts, states, blk)) : __feed([x, idx], i + 1, counts, states, blk)
      when :take
        throw :__lazy_done if f == 0
        counts[i] += 1
        __feed(x, i + 1, counts, states, blk)
        throw :__lazy_done if counts[i] >= f
      when :drop
        counts[i] += 1
        __feed(x, i + 1, counts, states, blk) if counts[i] > f
      when :uniq
        states[i] ||= {}
        unless states[i].key?(x)
          states[i][x] = true
          __feed(x, i + 1, counts, states, blk)
        end
      end
    end
    def first(n = nil)
      res = []
      catch(:__lazy_first) do
        each { |x| res << x; throw :__lazy_first if res.size >= (n || 1) }
      end
      n ? res : res[0]
    end
    def to_a = (a = []; each { |x| a << x }; a)
    alias force to_a
    def eager = to_a.each
    def inspect = "#<Enumerator::Lazy: ...>"
  end
end

class Set
  include Enumerable
  def initialize(enum = nil, &b)
    @hash = {}
    enum&.each { |x| @hash[b ? b.call(x) : x] = true }
  end
  def self.[](*a) = new(a)
  def add(x) = (@hash[x] = true; self)
  alias << add
  def add?(x) = (@hash.key?(x) ? nil : add(x))
  def delete(x) = (@hash.delete(x); self)
  def delete?(x) = (@hash.key?(x) ? delete(x) : nil)
  def include?(x) = @hash.key?(x)
  alias member? include?
  alias === include?
  alias contain? include?
  def size = @hash.size
  alias length size
  alias count_fast size
  def empty? = @hash.empty?
  def each(&b)
    return enum_for(:each) unless b
    @hash.each_key(&b)
    self
  end
  def to_a = @hash.keys
  def dup = Set.new(to_a)
  def |(o) = Set.new(to_a + o.to_a)
  alias union |
  alias + |
  def &(o) = Set.new(to_a.select { |x| o.include?(x) })
  alias intersection &
  def -(o) = Set.new(to_a.reject { |x| o.include?(x) })
  alias difference -
  def ^(o) = (self | o) - (self & Set.new(o))
  def subset?(o) = all? { |x| o.include?(x) }
  alias <= subset?
  def superset?(o) = o.all? { |x| include?(x) }
  alias >= superset?
  def proper_subset?(o) = subset?(o) && size < o.size
  alias < proper_subset?
  def proper_superset?(o) = superset?(o) && size > o.size
  alias > proper_superset?
  def disjoint?(o) = none? { |x| o.include?(x) }
  def intersect?(o) = !disjoint?(o)
  def merge(*es) = (es.each { |e| e.each { |x| add(x) } }; self)
  def ==(o) = o.is_a?(Set) && size == o.size && subset?(o)
  def hash = to_a.map(&:hash).sort.hash
  def eql?(o) = self == o
  def clear = (@hash.clear; self)
  def map!(&b) = (a = to_a.map(&b); @hash = {}; a.each { |x| add(x) }; self)
  def select!(&b) = (n = size; @hash.select! { |k, _| b.call(k) }; n == size ? nil : self)
  alias filter! select!
  def reject!(&b) = (n = size; @hash.reject! { |k, _| b.call(k) }; n == size ? nil : self)
  def delete_if(&b) = (reject!(&b); self)
  def keep_if(&b) = (select!(&b); self)
  def to_set = self
  def freeze = (@hash.freeze; super)
  def min(*a, &b) = to_a.min(*a, &b)
  def max(*a, &b) = to_a.max(*a, &b)
  def sort(&b) = to_a.sort(&b)
  def group_by(&b) = to_a.group_by(&b)
  def inspect = "#<Set: {#{to_a.map(&:inspect).join(', ')}}>"
  alias to_s inspect
end
