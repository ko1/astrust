# A baked site `x.f(y)` with A#f compiled in.  A miss after a method-serial bump refills the cache with
# another entry; that entry re-enters the site before call_spec_out resets spec_klass.  A stale spec_klass
# ran the baked A#f over the other entry's nodes (SIGSEGV in call_slow, or wrong values).
# 1: another class's f.  2: A#f redefined (same class).  CRuby: [2, [7, 3], 1] / [2, [7, -3], 1].
class A; def f(q) = q.size; end
class B; def f(q) = [q, g(A.new, "abc")]; end
def g(x, y) = x.f(y)
r = []
r << g(A.new, "ab")
def zz1; end
r << g(B.new, 7)
def zz2; end
r << g(A.new, "x")
p r

class C; def f(q) = q.size; end
def h(x, y) = x.f(y)
r = []
r << h(C.new, "ab")
class C; def f(q) = Integer === q ? [q, h(C.new, "abc")] : -q.size; end
r << h(C.new, 7)
class C; def f(q) = q.size; end
r << h(C.new, "x")
p r
