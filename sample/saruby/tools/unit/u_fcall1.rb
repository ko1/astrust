class C
  def f(x); x + 1; end
  def run
    i = 0; s = 0
    while i < 3_000_000
      s = f(s); s = f(s); s = f(s); s = f(s); s = f(s)
      i += 1
    end
    s
  end
end
p C.new.run
