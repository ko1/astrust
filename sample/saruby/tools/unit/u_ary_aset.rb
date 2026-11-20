a = Array.new(256, 0)
i = 0
while i < 3_000_000
  a[i & 255] = i; a[(i + 7) & 255] = i
  i += 1
end
p a[0]
