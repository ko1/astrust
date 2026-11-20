# Random differential cases (same TSV format as gen.rb). Usage: ruby fuzz.rb [count] [seed]
N = (ARGV[0] || 2000).to_i
RICH = ARGV[2] == "rich"
SIMPLE = RICH ? %w[a b c é あ . \\w \\W \\d \\h \\s [ab] [^a] [a-c] [é-ê] [[:alpha:]] [[:^digit:]] \\p{L} \\p{Hiragana} [a-z&&[^b]] \\x41 \\u3042 x ^ $ \\b \\B \\A \\z \\Z \\K \\R \\X \\G \\n \\t ab ss] :
  %w[a b c a b . \\w \\d [ab] [^a] [a-c] \\s x ^ $ \\b \\B \\A \\z \\Z]
SUBJ = RICH ? ["a", "b", "c", "A", "é", "É", "あ", "ア", "\n", "\r\n", " ", "1", "ß", "S", "e\u0301", "_"] : %w[a b c a b A B \n x]
Regexp.timeout = 0.5
srand((ARGV[1] || 1).to_i)
def atom(d)
  r = rand(100)
  if d > 3 || r < 35
    SIMPLE.sample
  elsif r < 55 then "(" + alt(d + 1) + ")"
  elsif r < 65 then "(?:" + alt(d + 1) + ")"
  elsif r < 70 then "(?=" + alt(d + 1) + ")"
  elsif r < 74 then "(?!" + alt(d + 1) + ")"
  elsif r < 78 then "(?<=" + %w[a b ab c ba].sample + ")"
  elsif r < 81 then "(?<!" + %w[a b ab|c].sample + ")"
  elsif r < 85 then "(?>" + alt(d + 1) + ")"
  elsif r < 90 then "\\1"
  elsif r < 93 then "(?i:" + alt(d + 1) + ")"
  elsif RICH && r < 95 then "(?<n>" + alt(d + 1) + ")"
  elsif RICH && r < 96 then "\\k<n>"
  elsif RICH && r < 98 then "(?(1)" + seq(d + 1) + "|" + seq(d + 1) + ")"
  else %w[A B C].sample
  end
end
def quant(a)
  return a if a =~ /\A(\^|\$|\\[bBAzZKG])\z/
  case rand(100)
  when 0...55 then a
  when 55...65 then a + "*"
  when 65...72 then a + "+"
  when 72...78 then a + "?"
  when 78...82 then a + "*?"
  when 82...85 then a + "+?"
  when 85...87 then a + "??"
  when 87...89 then a + "*+"
  when 89...91 then a + "++"
  when 91...95 then a + "{#{rand(3)},#{rand(3) + 2}}"
  when 95...97 then a + "{#{rand(3)}}"
  else a + "{#{rand(2)},}?"
  end
end
def seq(d) = (1..rand(1..4)).map { quant(atom(d)) }.join
def alt(d) = (1..(rand(4) == 0 ? 2 : 1)).map { seq(d) }.join("|")
out = []
N.times do
  pat = alt(0)
  subj = (0...rand(0..(rand(4) == 0 ? 30 : 10))).map { SUBJ.sample }.join
  opts = [0, 0, 0, Regexp::IGNORECASE, Regexp::MULTILINE].sample
  start = subj.b.size.zero? ? 0 : subj[0, rand(0..subj.size)].bytesize
  start = 0 if rand(2) == 0
  mode = rand(5) == 0 ? "m" : "s"
  begin
    re = Regexp.new(mode == "m" ? "\\G(?:#{pat})" : pat, opts)
  rescue RegexpError => e
    out << [pat.b.unpack1("H*"), opts, subj.b.unpack1("H*"), start, mode, "E:" + e.message.sub(/: \/.*\z/m, "")].join("\t")
    next
  end
  m = begin
    subj.byteindex(re, start) && $~
  rescue => e
    next
  end
  exp = m ? (0...m.size).map { |i| o = m.byteoffset(i); o[0] ? o.join(",") : "-" }.join(";") : "N"
  names = re.named_captures.map { |n, v| "#{n}=#{v.join(",")}" }.join(" ")
  row = [pat.b.unpack1("H*"), opts, subj.b.unpack1("H*"), start, mode, exp]
  row << names unless names.empty?
  out << row.join("\t")
end
puts out
