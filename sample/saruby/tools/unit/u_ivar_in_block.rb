class C
  def initialize; @a = 1; @b = 2; @c = 3; @d = 4; @e = 5; end
  def run
    s = 0
    0.step(3_000_000 * 8 - 8, 8) do
      s += @a; s += @b; s += @c; s += @d; s += @e
    end
    s
  end
end
p C.new.run
