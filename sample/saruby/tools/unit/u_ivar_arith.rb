class C
  def initialize; @a = 0; @b = 0; @c = 0; @d = 0; @e = 0; end
  def run
    i = 0
    while i < 3_000_000
      @a += 1; @b += i; @c -= 1; @d += 2; @e -= i
      i += 1
    end
    @a + @b + @c + @d + @e
  end
end
p C.new.run
