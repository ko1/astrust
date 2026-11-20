class C
  def initialize; @a = 1; @b = 2; @c = 3; @d = 4; @e = 5; end
  def run
    i = 0
    while i < 3_000_000
      @a = i; @b = i; @c = i; @d = i; @e = i
      i += 1
    end
    @a + @e
  end
end
p C.new.run
