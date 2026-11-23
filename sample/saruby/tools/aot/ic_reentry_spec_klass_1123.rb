# A call site re-entered by recursion refills its cache (method serial bumped in between).  With the cache
# as `&mut CallCache` LLVM kept the outer call_spec_out's pre-call klass (noalias), so spec_klass named A while
# the cache held B's entry: the third call ran B#f for an A.  CRuby: [:A, :A, :A, :B].
class A
  def f(o, d)
    if d > 0
      Object.define_method(:"zz#{d}#{rand(1 << 30)}") {}
      g(o, self, d - 1)
    end
    :A
  end
end
class B; def f(o, d) = :B; end
def g(x, y, d) = x.f(y, d)
a = A.new; b = B.new
r = []
r << g(a, b, 0)
r << g(a, b, 1)
r << g(a, b, 0)
r << g(b, a, 0)
p r
