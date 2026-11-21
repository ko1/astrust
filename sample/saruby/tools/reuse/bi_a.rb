# the site sees Arrays
def f(x) = x.size
def run(o) = (s = 0; i = 0; while i < 3_000_000; s += f(o); i += 1; end; s)
p run([1, 2, 3])
