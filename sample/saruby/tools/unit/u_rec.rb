class C
  # 10 calls per d(9): one from run, nine recursive
  def d(n) = n == 0 ? 0 : d(n - 1)
  def run
    i = 0; s = 0
    while i < 3_000_000
      s += d(9)
      i += 1
    end
    s + i
  end
end
p C.new.run
