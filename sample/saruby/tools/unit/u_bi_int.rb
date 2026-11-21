i = 0; t = nil
while i < 3_000_000
  t = i.even?; t = i.odd?; t = i.zero?
  i += 1
end
p t
