# program A of the shared-callee pair: recursive methods called from the top level
def fib(n) = n < 2 ? n : fib(n - 1) + fib(n - 2)

def tak(x, y, z) = y < x ? tak(tak(x - 1, y, z), tak(y - 1, z, x), tak(z - 1, x, y)) : z

p fib(27)
p tak(18, 12, 6)
