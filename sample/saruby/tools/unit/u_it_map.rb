a = Array.new(1000) { |i| i }
b = nil
3000.times { b = a.map { |x| x + 1 } }
p b.size
