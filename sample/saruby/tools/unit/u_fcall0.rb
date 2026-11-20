class C
  def initialize; @a = 1; end
  def f; @a; end
  def run
    i = 0; s = 0
    while i < 3_000_000
      s += f; s += f; s += f; s += f; s += f
      i += 1
    end
    s
  end
end
p C.new.run
