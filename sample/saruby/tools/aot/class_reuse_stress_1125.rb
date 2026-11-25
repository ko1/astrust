# A class allocated in the slot of a freed singleton class / module must not hit any cache keyed by
# the dead class's address: the global method cache, call sites, the send table, `when` memos.
def mk
  o = Object.new
  def o.to_int = 2
  o.inspect
  nil
end

# the first report, under GC.stress: K0.inspect resolved to the dead singleton's Kernel#inspect
GC.stress = true
3.times do |i|
  mk
  GC.start
  k = Class.new
  Object.const_set(:"StressK#{i}", k)
  p [k.inspect, k.name, k.to_s, k.method(:inspect).owner]
end
GC.stress = false

def call(x) = (x.hello rescue :nomethod)
def snd(x) = (x.send(:hello) rescue :nomethod)
def pub(x) = (x.public_send(:hello) rescue :nomethod)

bad = Hash.new(0)
4.times do
  objs = Array.new(200) { Object.new }
  objs.each { |o| def o.hello = :stale }
  objs.each { |o| call(o); snd(o); pub(o); o.respond_to?(:hello); o.method(:hello) }
  objs = nil
  GC.start
  ks = Array.new(200) { Class.new }
  ks.each do |k|
    bad[:call] += 1 if call(k) != :nomethod
    bad[:send] += 1 if snd(k) != :nomethod
    bad[:public_send] += 1 if pub(k) != :nomethod
    bad[:respond_to] += 1 if k.respond_to?(:hello)
    bad[:method] += 1 if (k.method(:hello) rescue nil)
  end
end
p [:singleton, bad.sum { _2 }]

# `when` memo and kind_of? on a dead module's address
def kind(x, m) = case x when m then :yes else :no end
bad = 0
4.times do
  ms = Array.new(100) { Module.new }
  objs = ms.map { |m| Object.new.extend(m) }
  objs.each_with_index { |o, i| kind(o, ms[i]); o.kind_of?(ms[i]) }
  objs = ms = nil
  GC.start
  ks = Array.new(100) { Class.new }
  mods = Array.new(100) { Module.new }
  ks.each { |k| mods.each_slice(25) { |s| bad += 1 if kind(k, s[0]) != :no || k.kind_of?(s[0]) } }
end
p [:module_kind, bad]
p :done
