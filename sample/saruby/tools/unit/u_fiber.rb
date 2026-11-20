f = Fiber.new do
  i = 0
  while true
    Fiber.yield i
    i += 1
  end
end
s = 0; i = 0
while i < 600_000
  s += f.resume
  i += 1
end
p s
