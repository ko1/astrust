class C
  def initialize; @a = 1; @b = 2; @c = 3; @d = 4; @e = 5; end
  def run
    i = 0; s = 0
    while i < 3_000_000
      s += @a; s += @b; s += @c; s += @d; s += @e
      i += 1
    end
    s
  end
end
p C.new.run
