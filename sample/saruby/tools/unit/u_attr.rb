class C
  attr_reader :a
  attr_writer :b
  def initialize; @a = 1; @b = 0; end
end
o = C.new; i = 0; s = 0
while i < 3_000_000
  s += o.a; o.b = i; s += o.a
  i += 1
end
p s
