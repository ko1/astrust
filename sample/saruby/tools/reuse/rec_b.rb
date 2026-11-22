# program B: the same recursive methods as rec_a.rb, called from other methods and a block
def tak(x, y, z) = y < x ? tak(tak(x - 1, y, z), tak(y - 1, z, x), tak(z - 1, x, y)) : z

def fib(n) = n < 2 ? n : fib(n - 1) + fib(n - 2)

def fibs(k) = k.times.map { |i| fib(22 + i) }.sum

def taks = [tak(18, 12, 6), tak(12, 8, 4)]

p fibs(6)
p taks
