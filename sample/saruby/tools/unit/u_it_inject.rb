s = 0
a = Array.new(1000) { |i| i }
3000.times { s = a.inject(0) { |m, x| m + x } }
p s
