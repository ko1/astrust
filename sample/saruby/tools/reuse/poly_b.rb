# the same site calls Q#m (another body)
class P; def m = 1; end
class Q; def m = 2; end
def f(x) = x.m
def run(o) = (s = 0; i = 0; while i < 3_000_000; s += f(o); i += 1; end; s)
p run(Q.new)
