i = 0; a = []
while i < 3_000_000
  a << i
  a.clear if a.size > 256
  i += 1
end
p a.size
