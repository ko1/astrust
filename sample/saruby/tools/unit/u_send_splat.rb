class C
  def initialize; @t = [[:f, 1], [:g, 2]]; end
  def f(x) = x
  def g(x) = x + 1
  def run
    i = 0; s = 0
    while i < 3_000_000
      s += send(*@t[i & 1])
      i += 1
    end
    s
  end
end
p C.new.run
