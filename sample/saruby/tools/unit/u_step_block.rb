s = 0
0.step(3_000_000 * 8 - 8, 8) do |i|
  s += i
end
p s
