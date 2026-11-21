a = [1, 2, 3, 4, 5]
i = 0; t = nil
while i < 3_000_000
  t = a.size; t = a.empty?; t = a.first
  i += 1
end
p t
