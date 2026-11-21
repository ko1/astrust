class C
  def adder(n) = ->(x) { x + n }
  def run
    i = 0; s = nil
    while i < 3_000_000
      s = adder(i)
      i += 1
    end
    s.call(1)
  end
end
p C.new.run
