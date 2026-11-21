i = 0; a = []
while i < 3_000_000
  a.push(i)
  a = [] if (i & 255) == 0
  i += 1
end
p a.size
