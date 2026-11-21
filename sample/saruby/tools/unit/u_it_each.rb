s = 0
a = Array.new(1000) { |i| i }
3000.times { a.each { |x| s += x } }
p s
