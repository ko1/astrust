class C
  def f(x:, y:) = x + y
  def run
    i = 0; s = 0
    while i < 3_000_000
      s = f(x: s, y: 1)
      i += 1
    end
    s
  end
end
p C.new.run
