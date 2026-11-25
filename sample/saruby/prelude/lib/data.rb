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
      k.singleton_class.send(:undef_method, :define)
      __saruby__.__ivset(k, :__members__, syms.freeze)
      syms.each { |m| k.define_method(m) { __saruby__.__ivget(self, m) } }
      class << k
        define_method(:new) do |*args, **kw, &blk|
          mem = __saruby__.data_members(self)
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
        def members = __saruby__.data_members(self).dup
      end
      k.class_eval(&blk) if blk
      k
    end
  end
  def __saruby__.data_members(k)
    k = k.superclass while __ivget(k, :__members__).nil?
    __ivget(k, :__members__)
  end
  # rb_struct_pos(name_only = true): a Symbol, a String or a #to_str names a member; [index or nil, the key]
  def __saruby__.data_pos(mem, k)
    unless k.is_a?(Symbol) || k.is_a?(String)
      s = String.try_convert(k)
      raise TypeError, "#{k.inspect} is not a symbol nor a string" unless s
      k = s
    end
    [mem.index(k.to_sym), k]
  end
  # rb_data_initialize_m: every key is looked at, then missing before unknown
  def initialize(**kw)
    mem = __saruby__.data_members(self.class)
    missing = mem.dup
    left = mem.size
    unknown = nil
    kw.each do |k, v|
      i, key = __saruby__.data_pos(mem, k)
      if i
        __saruby__.__ivset(self, mem[i], v)
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
    o.class.members.each { |m| __saruby__.__ivset(self, m, __saruby__.__ivget(o, m)) }
    freeze
  end
  private :initialize_copy
  def members = self.class.members
  def to_h(&b)
    h = {}
    members.each do |m|
      v = __saruby__.__ivget(self, m)
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
  def deconstruct = members.map { |m| __saruby__.__ivget(self, m) }
  def deconstruct_keys(keys)
    return to_h if keys.nil?
    raise TypeError, "wrong argument type #{keys.class} (expected Array or nil)" unless keys.is_a?(Array)
    h = {}
    mem = members
    return h if keys.size > mem.size
    keys.each do |k|
      i, k = __saruby__.data_pos(mem, k)
      break unless i
      h[k] = __saruby__.__ivget(self, mem[i])
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
  def __saruby__.data_paired(s, o, tag)
    seen = (__ivget(Data, tag) || __ivset(Data, tag, {}))
    key = [s.object_id, o.object_id]
    return true if seen.key?(key)
    seen[key] = true
    begin
      yield
    ensure
      seen.delete(key)
    end
  end
  def ==(o)
    return true if equal?(o)
    return false unless o.class == self.class
    __saruby__.data_paired(self, o, :__eq) { members.all? { |m| __saruby__.__ivget(self, m) == __saruby__.__ivget(o, m) } }
  end
  def eql?(o)
    return true if equal?(o)
    return false unless o.class == self.class
    __saruby__.data_paired(self, o, :__eql) { members.all? { |m| __saruby__.__ivget(self, m).eql?(__saruby__.__ivget(o, m)) } }
  end
  def hash = [self.class, *deconstruct].hash
  def inspect
    # rb_class_path, not #name; a path under an anonymous module is left out
    name = Module.instance_method(:name).bind_call(self.class)
    name = nil if name&.start_with?("#")
    seen = (__saruby__.__ivget(Data, :__insp) || __saruby__.__ivset(Data, :__insp, {}))
    return "#<data #{name || Module.instance_method(:to_s).bind_call(self.class)}:...>" if seen.key?(object_id)
    seen[object_id] = true
    begin
      # rb_data_inspect: a member that is not a local / constant name shows as a Symbol
      parts = members.map do |m|
        v = __saruby__.__ivget(self, m)
        "#{m.match?(/\A(?:[A-Za-z_]|[^\x00-\x7F])(?:\w|[^\x00-\x7F])*\z/) ? m : m.inspect}=#{v.inspect}"
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
