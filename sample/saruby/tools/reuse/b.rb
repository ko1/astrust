# program B: the same methods as a.rb at other lines, with other code around them
x = [1, 2, 3].map { _1 * 2 }

def tag(s) = "<" + s + ">"


def total(a) = a.map { |x| x * 1.5 }.sum

class Pt
  def initialize(x, y) = (@x = x; @y = y)
  def norm1 = @x.abs + @y.abs
  def dist2 = @x * @x + @y * @y
  def scale(k: 2) = Pt.new(@x * k, @y * k)
end
def fib(n) = n < 2 ? n : fib(n - 1) + fib(n - 2)

def sum_sq(n)
  s = 0
  n.times { |i| s += i * i }
  s
end

def sum_cube(n)
  s = 0
  n.times { |i| s += i * i * i }
  s
end

p fib(30), x
p sum_sq(1_000_000), sum_cube(10)
p Pt.new(3, 4).scale(k: 3).dist2, Pt.new(1, -2).norm1
p total([1, 2, 3] * 30000)
p tag("b")
begin
  fib(nil)
rescue => e
  puts e.backtrace.first(3)
end
