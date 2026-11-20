a = Array.new(256) { |i| i }
i = 0; s = 0
while i < 3_000_000
  s += a[i & 255]; s += a[(i + 7) & 255]; s += a[(i + 13) & 255]
  i += 1
end
p s
