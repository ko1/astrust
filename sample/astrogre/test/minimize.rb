# Delta-debug failing cases: ruby minimize.rb < failing.tsv (rows in the gen.rb/fuzz.rb format)
require_relative "cases_lib"
BIN = File.join(__dir__, "target/release/regex_test")
def fails?(p, s, o, st, md)
  r = row(p, s, o, st, md) rescue (return false)
  return false if r.split("\t")[5].start_with?("E:")
  out = IO.popen([BIN], "r+") { |io| io.puts r; io.close_write; io.read }
  out.include?("FAIL")
end
$stdin.each_line do |line|
f = line.chomp.split("\t")
pat, opts, subj, start, mode = [f[0]].pack("H*").force_encoding("UTF-8"), f[1].to_i, [f[2]].pack("H*").force_encoding("UTF-8"), f[3].to_i, f[4]
next puts("does not fail") unless fails?(pat, subj, opts, start, mode)
loop do
  changed = false
  (1..pat.size).to_a.reverse.each do |len|
    i = 0
    while i + len <= pat.size
      c = pat[0, i] + pat[i + len..]
      if fails?(c, subj, opts, start, mode)
        pat = c; changed = true
      else
        i += 1
      end
    end
  end
  break unless changed
end
puts IO.popen([BIN], "r+") { |io| io.puts row(pat, subj, opts, start, mode); io.close_write; io.read }
end
