# ivars set in the order @a, @b
class P
  def initialize = (@a = 1; @b = 2)
  def sum = @a + @b
end
def run(o) = (s = 0; i = 0; while i < 3_000_000; s += o.sum; i += 1; end; s)
p run(P.new)
