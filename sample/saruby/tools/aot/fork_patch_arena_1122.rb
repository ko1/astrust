# Code bound after fork must stay each process's own (knucleotide: Process.fork workers).
# The pipes fix the order: each side binds new bodies while the other's are live.
def parent_work(n)
  s = 0
  i = 0
  while i < n
    s += i * 3 + (i % 7)
    i += 1
  end
  [s, n].map { |x| x + 1 }.sum
end

def parent_more(a) = a.each_slice(3).map { |x| x.sum * 2 }.select(&:even?).size
def parent_last(h) = h.sort_by { |k, v| [-v, k] }.first(2)

def child_a(n) = (1..n).map { |i| i * 2 }.select { |x| x % 3 == 0 }.sum
def child_b(n) = (1..n).inject(0) { |s, i| s + i * i }
def child_c(s) = s.chars.map(&:upcase).join("-")
def child_d(n)
  t = Hash.new(0)
  n.times { |i| t[i % 5] += i }
  t.to_a.flatten.sum
end

to_child_r, to_child_w = IO.pipe
to_parent_r, to_parent_w = IO.pipe
pid = fork do
  to_child_w.close
  to_parent_r.close
  ok = []
  ok << (child_d(50) == 1235)
  to_parent_w.write("1")             # the parent now binds its bodies
  to_child_r.read(1)
  ok << (child_d(50) == 1235)        # after the parent bound its own
  ok << ([child_a(100), child_b(100), child_c("abc")] == [3366, 338350, "A-B-C"])
  to_parent_w.write(ok.all? ? "ok" : "ng")
  to_parent_w.close
  exit!(ok.all? ? 0 : 1)
end
to_child_r.close
to_parent_w.close
to_parent_r.read(1)
p parent_work(1000)
p parent_more((1..30).to_a)
to_child_w.write("1")
to_child_w.close
p to_parent_r.read
Process.wait(pid)
p $?.exitstatus
p parent_work(1000)
p parent_more((1..30).to_a)
p parent_last({ "a" => 3, "b" => 5, "c" => 5 })
