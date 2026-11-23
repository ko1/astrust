# aot_fork_test: the bodies baked; a stale cache hit at a baked site runs another class's method
# A dead singleton class / module's slot reused by a new class: no method cache keyed by the class's
# address (the global (class, mid) cache, call-site caches) may answer for the new class.
def call(x) = (x.hello rescue :nomethod)

# global cache: entries of 200 dead singletons, then 200 new classes
bad_rt = bad_site = 0
5.times do
  objs = Array.new(200) { Object.new }
  objs.each { |o| def o.hello = :stale }
  objs.each { |o| o.respond_to?(:hello); call(o) }
  objs = nil
  GC.start
  ks = Array.new(200) { Class.new }
  ks.each { |k| bad_rt += 1 if k.respond_to?(:hello); bad_site += 1 if call(k) != :nomethod }
end
p [:global_cache, bad_rt, bad_site]

# call-site caches: site j primed with singleton j at the final method serial (a later miss fills only
# the polymorphic ways, so the primary entry keeps it)
M = 150
M.times { |j| eval "def call_#{j}(x) = (x.hello rescue :nomethod)" }
bad = 0
3.times do
  objs = Array.new(M) { Object.new }
  objs.each { |o| def o.hello = :stale }
  objs.each_with_index { |o, j| send(:"call_#{j}", o) }
  objs = nil
  GC.start
  ks = Array.new(M) { Class.new }
  ks.each { |k| M.times { |j| bad += 1 if send(:"call_#{j}", k) != :nomethod } }
end
p [:site_cache, bad]

# a stale entry's body is a block whose Proc died with the singleton class
res = Hash.new(0)
5.times do |t|
  objs = Array.new(200) { Object.new }
  objs.each_with_index { |o, i| s = "p#{t}-#{i}"; o.define_singleton_method(:hello) { s } }
  objs.each { |o| o.respond_to?(:hello); call(o) }
  objs = nil
  GC.start
  ks = Array.new(200) { Class.new }
  junk = Array.new(200) { |j| [j, j.to_s] }
  ks.each { |k| r = call(k); res[r.is_a?(Symbol) ? r : :stale] += 1 }
end
p res

# anonymous modules die too
bad = 0
5.times do
  ms = Array.new(100) { Module.new { def hello = :stale } }
  objs = ms.map { |m| Object.new.extend(m) }
  objs.each { |o| call(o) }
  objs = ms = nil
  GC.start
  ks = Array.new(100) { Class.new }
  ks.each { |k| bad += 1 if call(k) != :nomethod }
end
p [:module, bad]

# the first report: a named class's inspect resolved to Kernel#inspect
def mk
  o = Object.new
  def o.to_int = 2
  o.inspect
  nil
end
3.times do |i|
  mk
  GC.start
  k = Class.new
  Object.const_set(:"ReuseK#{i}", k)
  p [k.inspect, k.name, k.method(:inspect).owner]
end
