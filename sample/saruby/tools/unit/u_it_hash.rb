s = 0
h = {}
1000.times { |i| h[i] = i }
3000.times { h.each { |k, v| s += v } }
p s
