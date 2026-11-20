# Delta-debug a hanging case: ruby minhang.rb < row.tsv (our engine must time out; Ruby must accept the pattern)
require_relative "cases_lib"
BIN = File.join(__dir__, "target/release/regex_test")
def hangs?(p, s, o, st, md)
  r = row(p, s, o, st, md) rescue (return false)
  return false if r.split("\t")[5].start_with?("E:")
  IO.popen(["timeout", "1", BIN], "r+") { |io| io.puts r; io.close_write; io.read }
  $?.exitstatus == 124
end
Regexp.timeout = 0.5
$stdin.each_line do |line|
  f = line.chomp.split("\t")
  pat, opts, subj, start, mode = [f[0]].pack("H*").force_encoding("UTF-8"), f[1].to_i, [f[2]].pack("H*").force_encoding("UTF-8"), f[3].to_i, f[4]
  loop do
    changed = false
    (1..pat.size).to_a.reverse.each do |len|
      i = 0
      while i + len <= pat.size
        c = pat[0, i] + pat[i + len..]
        if hangs?(c, subj, opts, start, mode) then pat = c; changed = true else i += 1 end
      end
    end
    break unless changed
  end
  p [pat, subj, opts, start, mode]
end
