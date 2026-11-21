# the same sum, ivars set in the order @b, @a (other slots)
class P
  def initialize = (@b = 2; @a = 1)
  def sum = @a + @b
end
def run(o) = (s = 0; i = 0; while i < 3_000_000; s += o.sum; i += 1; end; s)
p run(P.new)
