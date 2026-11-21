class C
  def run
    f = ->(x) { x + 1 }
    i = 0; s = 0
    while i < 3_000_000
      s = f.call(s)
      i += 1
    end
    s
  end
end
p C.new.run
