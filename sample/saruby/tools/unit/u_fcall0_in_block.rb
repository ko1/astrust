class C
  def initialize; @a = 1; end
  def f; @a; end
  def run
    s = 0
    0.step(3_000_000 * 8 - 8, 8) do
      s += f; s += f; s += f; s += f; s += f
    end
    s
  end
end
p C.new.run
