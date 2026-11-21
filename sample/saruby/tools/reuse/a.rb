# program A: bakes the store
def fib(n) = n < 2 ? n : fib(n - 1) + fib(n - 2)

class Pt
  def initialize(x, y) = (@x = x; @y = y)
  def dist2 = @x * @x + @y * @y
  def scale(k: 2) = Pt.new(@x * k, @y * k)
end

def sum_sq(n)
  s = 0
  n.times { |i| s += i * i }
  s
end

def total(a) = a.map { |x| x * 1.5 }.sum

def tag(s) = "<" + s + ">"

p fib(27)
p sum_sq(300_000)
p Pt.new(3, 4).scale(k: 3).dist2
p total([1, 2, 3] * 10000)
p tag("a")
