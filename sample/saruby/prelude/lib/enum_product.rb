# Enumerator::Product (loaded on the first reference to the constant, boot.rs lazy_const)
class Enumerator
  class Product < Enumerator
    undef_method :feed, :next, :next_values, :peek, :peek_values
    def initialize(*enums, **kw)
      unless kw.empty?
        ks = kw.keys.map(&:inspect).join(", ")
        raise ArgumentError, "unknown keyword#{kw.size > 1 ? 's' : ''}: #{ks}"
      end
      __saruby__.__ivinit(self, :__enums, enums.freeze)
      self
    end
    def initialize_copy(o)
      return self if equal?(o)
      raise TypeError, "initialize_copy should take same class object" unless o.class == self.class
      oe = __saruby__.__ivget(o, :__enums)
      raise ArgumentError, "uninitialized product" unless oe
      __saruby__.__ivinit(self, :__enums, oe)
      self
    end
    private :initialize_copy
    def each(&b)
      return self unless b
      __saruby__.product_rec(__saruby__.__ivget(self, :__enums), 0, [], &b)
      self
    end
    # enum_product_total_size
    def size
      sizes = __saruby__.__ivget(self, :__enums).map { |e| s = e.respond_to?(:size) ? e.size : nil; return s if s == 0; s }
      sizes.inject(1) do |t, s|
        return s if s.nil? || (s.is_a?(Float) && s.infinite?)
        return nil unless s.is_a?(Integer)
        t * s
      end
    end
    def rewind = (__saruby__.__ivget(self, :__enums).each { |e| e.rewind if e.respond_to?(:rewind) }; self)
    def inspect
      return "#<#{self.class}: uninitialized>" unless __saruby__.__ivget(self, :__enums)
      seen = (__saruby__.__ivget(Enumerator::Product, :__insp) || __saruby__.__ivset(Enumerator::Product, :__insp, {}))
      return "#<#{self.class}: ...>" if seen.key?(object_id)
      seen[object_id] = true
      begin
        "#<#{self.class}: #{__saruby__.__ivget(self, :__enums).inspect}>"
      ensure
        seen.delete(object_id)
      end
    end
    alias to_s inspect
  end
end

def __saruby__.product_rec(es, i, acc, &b)
  if i == es.size
    b.call(acc.dup)
  else
    es[i].each_entry { |v| acc.push(v); product_rec(es, i + 1, acc, &b); acc.pop }
  end
end
