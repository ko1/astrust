f = 12.75
i = 0; t = nil
while i < 3_000_000
  t = f.to_i; t = f.to_i; t = f.to_i
  i += 1
end
p t
