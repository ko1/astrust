class C
  def f(x) = x
  private def g(x) = x + 1
  def run
    i = 0; s = 0
    while i < 3_000_000
      s += send(:f, i) + __send__(:g, 1)
      i += 1
    end
    s
  end
end
p C.new.run
