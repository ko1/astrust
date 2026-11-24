# Data (loaded on the first reference to the constant, boot.rs lazy_const)
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
  # rb_struct_pos(name_only = true): a Symbol, a String or a #to_str names a member; [index or nil, the key]
  def self.__pos(mem, k)
    unless k.is_a?(Symbol) || k.is_a?(String)
      s = String.try_convert(k)
      raise TypeError, "#{k.inspect} is not a symbol nor a string" unless s
      k = s
    end
    [mem.index(k.to_sym), k]
  end
  # rb_data_initialize_m: every key is looked at, then missing before unknown
  def initialize(**kw)
    mem = self.class.__members
    missing = mem.dup
    left = mem.size
    unknown = nil
    kw.each do |k, v|
      i, key = Data.__pos(mem, k)
      if i
        Enumerable.__ivset(self, mem[i], v)
        if missing[i] then missing[i] = nil; left -= 1 end
      else
        (unknown ||= []) << key
      end
    end
    freeze
    if left > 0
      missing.compact!
      raise ArgumentError, "missing keyword#{left > 1 ? 's' : ''}: #{missing.map(&:inspect).join(', ')}"
    end
    if unknown
      raise ArgumentError, "unknown keyword#{unknown.size > 1 ? 's' : ''}: #{unknown.map(&:inspect).join(', ')}"
    end
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
      i, k = Data.__pos(mem, k)
      break unless i
      h[k] = Enumerable.__ivget(self, mem[i])
    end
    h
  end
  # rb_data_with: allocate + initialize, not .new
  def with(*args, **kw)
    raise ArgumentError, "wrong number of arguments (given #{args.size}, expected 0)" unless args.empty?
    return self if kw.empty?
    o = self.class.allocate
    o.__send__(:initialize, **to_h.merge(kw))
    o
  end
  # rb_exec_recursive_paired: a pair met again compares equal
  def __paired(o, tag)
    seen = (Enumerable.__ivget(Data, tag) || Enumerable.__ivset(Data, tag, {}))
    key = [object_id, o.object_id]
    return true if seen.key?(key)
    seen[key] = true
    begin
      yield
    ensure
      seen.delete(key)
    end
  end
  private :__paired
  def ==(o)
    return true if equal?(o)
    return false unless o.class == self.class
    __paired(o, :__eq) { members.all? { |m| Enumerable.__ivget(self, m) == Enumerable.__ivget(o, m) } }
  end
  def eql?(o)
    return true if equal?(o)
    return false unless o.class == self.class
    __paired(o, :__eql) { members.all? { |m| Enumerable.__ivget(self, m).eql?(Enumerable.__ivget(o, m)) } }
  end
  def hash = [self.class, *deconstruct].hash
  def inspect
    # rb_class_path, not #name; a path under an anonymous module is left out
    name = Module.instance_method(:name).bind_call(self.class)
    name = nil if name&.start_with?("#")
    seen = (Enumerable.__ivget(Data, :__insp) || Enumerable.__ivset(Data, :__insp, {}))
    return "#<data #{name || Module.instance_method(:to_s).bind_call(self.class)}:...>" if seen.key?(object_id)
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
