class C
  def y1; yield 1; end
  def run
    i = 0; s = 0
    while i < 3_000_000
      y1 { |v| s += v }
      i += 1
    end
    s
  end
end
p C.new.run
