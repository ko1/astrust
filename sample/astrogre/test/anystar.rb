# Cases around Onigmo's implicit anchor for a leading greedy `.*` (Bug #22494, CRuby 4.0.10):
# zero-width prefixes x `.*` bodies x m/non-m x subjects x every start offset. Same TSV as gen.rb.
#   ruby anystar.rb          rows with CRuby's results
#   ruby anystar.rb oracle   only rows where CRuby differs from trying each start with \G
require_relative "cases_lib"
PRE = ["", "(?=b)", "(?=(b))", "(?!a)", "(?<=a)", "(?<!a)", "(?<!a){2,3}", "(?=b)*", "(?=b)+", "(?=b){0,2}",
       "^", "$", "\\b", "\\B", "\\A", "\\G", "\\z", "\\Z", "\\K", "(?:^|\\b)", "(?:^|$)", "(?:(?=b))", "((?=b))",
       "(?i:\\b)", "\\K(?=b)", "(?=b)\\K", "()", "(?:)", "^(?=b)", "(?=b)^", "$\\b(?<!a){2,3}", "(?:\\b|)",
       "(?<n>(?=b))\\g<n>", "(?>(?=b))", "(?=a|b)", "(?=.)", "(?!$)", "\\b\\B", "(?=b)(?=b)"]
BODY = [".*", ".*+", ".*?", "(.*)", "(?>.*)", ".+", ".*b", ".*\\0", ".*x", "(?:.*)b?", ".*$", ".*\\z"]
SUBJ = ["ab", "axb", "aA", "xxx\nb\0", "aa\nb\0", "ab\n\0", "aa\0", "b", "", "a\nb\nx", "bb\nab"]
oracle = ARGV[0] == "oracle"
PRE.each do |pre|
  BODY.each do |body|
    pat = pre + body
    [0, Regexp::MULTILINE].each do |o|
      SUBJ.each do |s|
        (0..s.bytesize).each do |st|
          r = row(pat, s, o, st)
          next puts(r) unless oracle
          next if r.split("\t")[5].start_with?("E:")
          re = Regexp.new("\\G(?:#{pat})", o)
          ex = (st..s.bytesize).find { |i| s.byteindex(re, i) == i }
          cr = s.byteindex(Regexp.new(pat, o), st)
          puts "#{pat.inspect} opt=#{o} #{s.inspect} start=#{st}: ruby=#{cr.inspect} oracle=#{ex.inspect}" if cr != ex
        end
      end
    end
  end
end
