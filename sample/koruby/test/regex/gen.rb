# Generates expected results from CRuby for the regex engine tests.
# Output TSV: hex(pattern) opts hex(subject) start mode expected [names]
#   mode: s = search from start, m = anchored match at start
#   expected: N (no match) | E:<message> | "b,e;b,e;-" byte offsets per group (- = unset)
I = Regexp::IGNORECASE; X = Regexp::EXTENDED; M = Regexp::MULTILINE
C = []
def t(pat, subj, opts = 0, start = 0, mode = "s") = C << [pat, opts, subj, start, mode]

# literals & escapes
t 'abc', 'xxabcxx'
t 'abc', 'ababab'
t '', 'abc'
t '', ''
t 'a\tb\nc', "a\tb\nc"
t '\x41\x42', 'zAB'
t '\101\102', 'zAB'
t 'あ', 'xあy'
t '\u{3042 3044}', 'xあいy'
t '\u{1F600}', "a\u{1F600}"
t '\xe3\x81\x82', 'あ'
t '\cA\e\a\f\v\0', "\x01\e\a\f\v\0"
t '\.\*\+\?\(\)\[\]\{\}\|\^\$\\\/', '.*+?()[]{}|^$\/'
t '\-\#\ ', '-# '
t 'a{', 'a{'
t 'a{x}', 'a{x}'
t 'a{1,x}', 'a{1,x}'
t 'a{,}', 'a{,}'
t 'x{2,3', 'x{2,3'
t '{', '{'
t '}', 'a}'
t ']', 'a]'
t 'あい', 'xxあいう'
t 'é+', 'éééx'
t '\10', "a\x08"
t '\M-a', "\xe1".b
# dot
t 'a.c', 'abc a c a\nc'
t 'a.c', "a\nc"
t 'a.c', "a\nc", M
t '.+', "ab\ncd"
t '.+', "ab\ncd", M
t '...', 'aあb'
t '.', ''
# anchors
t '^b', "a\nb"
t 'a$', "a\nb"
t '^', "a\n"
t '^$', "a\n\nb"
t '$', "a\n"
t '^a', 'ba', 0, 1
t '\Aa', 'aa', 0, 1
t '\Aa', 'aa'
t 'a\z', "a\n"
t 'a\Z', "a\n"
t 'a\Z', "a\n\n"
t 'a\Z', 'a'
t '\bfoo\b', 'a foo b'
t '\bfoo\b', 'afoo'
t '\Boo\B', 'foo fooo'
t '\b', ''
t '\B', ''
t '\b', 'é a'
t '\Gab', 'ababxab'
t '\Gab', 'xab', 0, 1
t '\Gab', 'xab'
t 'a$', "ab\na"
t '(?m)^b.', "a\nb\nc"
# classes
t '\d+', 'abc123def'
t '\D+', '123abc456'
t '\w+', '  foo_Bar9 '
t '\W+', 'ab, cd'
t '\s+', "a \t\n\v\f\rb"
t '\S+', '  ab  '
t '\h+', 'xyzBEEFz'
t '\H+', '0aqqz1'
t '[abc]+', 'xxbcaxx'
t '[^abc]+', 'abcxyzabc'
t '[a-z]+', 'ABCdefGHI'
t '[a-cx-z]+', 'dddaczxbyd'
t '[a-c[x-z]]+', 'dddaczxbyd'
t '[a-z&&[^aeiou]]+', 'aeibcdfo'
t '[\w&&\D]+', '12ab_c3'
t '[[:alpha:]]+', '12abc34'
t '[[:digit:]]+', 'ab123c'
t '[[:space:]]+', "a \t\nb"
t '[[:upper:]]+', 'abCDe'
t '[[:lower:]]+', 'ABcdE'
t '[[:alnum:]]+', '--ab12--'
t '[[:punct:]]+', 'ab!?,.;cd'
t '[[:punct:]]+', 'a$+<=>^`|~b'
t '[[:xdigit:]]+', 'xx0aFgg'
t '[[:word:]]+', '--a_1--'
t '[[:cntrl:]]+', "a\x01\x02b"
t '[[:print:]]+', "\x01ab c\x02"
t '[[:graph:]]+', ' ab c'
t '[[:blank:]]+', "a \t\nb"
t '[[:^alpha:]]+', 'ab12cd'
t '[[:alpha:][:digit:]]+', '--a1--'
t '[[:alpha:]]+', 'éaβ1'
t '\p{Alpha}+', '12éaβ1'
t '\p{Digit}+', 'a123'
t '\p{Upper}+', 'aBCd'
t '\p{Lower}+', 'ABcdE'
t '\p{Alnum}+', '-a1-'
t '\p{Word}+', '-a_1-'
t '\p{Space}+', "a 　b"
t '\p{Punct}+', 'a!!b'
t '\p{L}+', '12abcあ3'
t '\p{Hiragana}+', 'カタあいうカ'
t '\p{Katakana}+', 'あいカタカナあ'
t '\p{Han}+', 'あ漢字か'
t '\P{Alpha}+', 'ab12cd'
t '\p{^Alpha}+', 'ab12cd'
t '[\p{Digit}x]+', 'ax12xb'
t '[^\d\s]+', '1 2 ab 3'
t '[\]]', 'a]'
t '[]a]+', 'b]a]'
t '[a-]+', 'x-a-'
t '[-a]+', 'x-a-'
t '[a\-z]+', 'bz-a'
t '[\w-]+', '!!a-b!!'
t '[.]', 'a.'
t '[\n]', "a\nb"
t '[\x41-\x43]+', 'zABCD'
t '[あ-ん]+', 'xあいうx'
t '[\^]', 'a^'
t '[a^]+', 'x^a^'
t '[\b]', "a\bb"
t '[a-c&&b-d]+', 'abcd'
t '[^a-z&&[^aeiou]]+', 'bcdaei'
t '[^]a]+', ']]abc'
t '[[:^space:]]+', '  ab  '
# errors
t '*a', 'a'
t 'a|*', 'a'
t '(*)', 'a'
t '+', 'a'
t '{2}', 'a'
t '[a', 'a'
t '[]', 'a'
t '[b-a]', 'a'
t '(a', 'a'
t 'a)', 'a'
t '\\', 'a'
t '\2(a)', 'a'
t '\1', 'a'
t '\p{Foo}', 'a'
t 'a{3,2}', 'a'
t 'a{100001}', 'a'
t '(?<=a+)b', 'ab'
t '(?<=a*)b', 'ab'
t '(?z)', 'a'
t '(?<1a>x)', 'a'
t '(?<>x)', 'a'
t '\k<zz>(?<a>x)', 'a'
t '(?<a>x)\1', 'xx'
t '(?<a>x)\k<1>', 'xx'
t '\x', 'a'
t '[[:foo:]]', 'a'
t '(?#abc', 'a'
t '[a-\d]', 'a'
# groups
t '(a)(b)(c)', 'abc'
t '(a(b(c)))', 'abc'
t '(a)|(b)', 'b'
t '(?:ab)+', 'ababab'
t '(a|ab)(c|bcd)(d*)', 'abcd'
t '((a)|b)+', 'ab'
t '(a)*', 'aaa'
t '(a)*', 'b'
t '(?:(a)|b)*', 'ab'
t '(?:a|(b))*', 'aaa'
t '(a*)+', 'b'
t '(a*)*', 'b'
t '(a|)+', 'aab'
t '(|a)+', 'aab'
t '(a*?)*', 'aa'
t '(a?)*?b', 'aab'
t '(?<first>a)(?<second>b)', 'ab'
t "(?'q'x+)y", 'xxy'
t '(?<n>a)(?<n>b)', 'ab'
t '(?<n>a)(b)(?<m>c)', 'abc'
t '(?<n>a|b)\k<n>', 'abb'
t "(?<n>a|b)\\k'n'", 'aba'
t '(?<n>a)|(?<n>b)\k<n>', 'bb'
t '(?<n>.)(?<n>.)\k<n>', 'abb'
t '(?<n>.)(?<n>.)\k<n>', 'aba'
t '(?<a>.)\k<a>', 'xyzzy'
# alternation
t 'a|b|c', 'xxc'
t 'ab|a', 'ab'
t 'a|ab', 'ab'
t '(a|ab)c', 'abc'
t 'cat|dog', 'hotdog'
t '|a', 'a'
t 'a|', 'b'
t '(?:a|b)*c', 'ababc'
t '(?:x|y|)+z', 'xyz'
# quantifiers
t 'a*', 'aaa'
t 'a*', 'baaa'
t 'a+', 'baaa'
t 'a?', 'baa'
t 'a{2}', 'aaaa'
t 'a{2,}', 'aaaa'
t 'a{,2}', 'aaaa'
t 'a{2,3}', 'aaaa'
t 'a{2,3}?', 'aaaa'
t 'a{2}?', 'a'
t 'a{2}?', 'aaa'
t 'a{2,}?', 'aaaa'
t 'a{,2}?', 'aaaa'
t 'a{0}b', 'ab'
t 'a{2}{3}', 'aaaaaaa'
t 'a**', 'aaa'
t 'a*?', 'aaa'
t 'a+?', 'aaa'
t 'a??b', 'ab'
t 'a*?b', 'aaab'
t '<.*>', '<a><b>'
t '<.*?>', '<a><b>'
t '<.+?>', '<><a>'
t 'a*+a', 'aaa'
t 'a++b', 'aaab'
t 'a?+a', 'a'
t '(?:ab){2,3}', 'abababab'
t '(?:ab){2,3}?', 'abababab'
t '(ab){2}', 'ababab'
t '(a|b){3}', 'abab'
t '(a|bc){2,}', 'abca'
t '(?:a{2}){2}', 'aaaaa'
t 'x(?:ab)?y', 'xy xaby'
t '(a+)+b', 'aaab'
t '(a+)+b', 'aaaa'
t '\d{3}-\d{4}', 'call 555-1234 now'
t '[a-z]+\d+', 'abc def123'
t '.*foo', 'xfooyfoo'
t '.*?foo', 'xfooyfoo'
t '.*$', "ab\ncd"
t '(.*)(\d+)', 'abc123'
t '(.*?)(\d+)', 'abc123'
t 'a.*b', 'axxbxxbxx'
t 'a.*?b', 'axxbxxbxx'
t '[^,]*,', 'ab,cd,'
t 'x*y*z*', ''
t '(?:a|b)*?b', 'aab'
t 'a{3,5}', 'aaaaaaaa'
t '(a{2,3}){2}', 'aaaaaa'
t '(a{1,2}?)(a*)', 'aaa'
# backrefs
t '(a)\1', 'aa'
t '(a)\1', 'ab'
t '(\w+)\s\1', 'hello hello'
t '(a)(b)?\2', 'a'
t '(a)|\1b', 'b'
t '(a*)b\1', 'aabaa'
t '(.)\1+', 'abbbc'
t '(?i)(a)\1', 'aA'
t '(a)\1', 'aA', I
t '(\w)(\w)\2\1', 'xabbay'
t '(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)\10', 'abcdefghijj'
t '(a)\k<-1>', 'aa'
t '(?:(a)|b)\1', 'aa'
# lookaround
t 'foo(?=bar)', 'foobaz foobar'
t 'foo(?!bar)', 'foobar foobaz'
t '(?<=\$)\d+', 'cost $42'
t '(?<!\$)\b\d+', '$42 17'
t '(?<=a|bc)d', 'bcd'
t '(?<=a|bc)d', 'xd ad'
t '(?<=ab|c)d', 'xcd'
t '(?<!ab|c)d', 'abd cd xd'
t '(?<=(a))b', 'ab'
t '(?=(a))a', 'a'
t '(?!(a))b', 'b'
t '(?=a)*a', 'a'
t 'a(?=b)', 'ab'
t '(?<=^|,)\w+', 'ab,cd'
t '(?<=\A)a', 'aa'
t '\w+(?=,)', 'ab,cd'
t '(?<=x)y*', 'xyy'
t '(?<=[a-c]{2})d', 'abd'
t '(?<=é)a', 'éa'
t '(?=.*\d)(?=.*[a-z])\w{6,}', 'abc123'
t '(?>a+)b', 'aaab'
t '(?>a+)a', 'aaaa'
t '(?>a|ab)c', 'abc'
t '(?>(a))b', 'ab'
t '(?>.*)x', 'abcx'
# options
t 'abc', 'xABCx', I
t '[a-z]+', 'ABC', I
t '[^a-z]+', 'ABC1', I
t '\w+', 'AbC', I
t 'ÄÖ', 'äö', I
t 'σ', 'Σ', I
t '(?i)abc', 'ABC'
t 'a(?i)bc', 'aBC'
t 'a(?i)bc', 'ABC'
t 'a(?i:b)c', 'aBc'
t 'a(?i:b)c', 'aBC'
t '(?i:a)B', 'Ab'
t '(?-i)a', 'A', I
t 'a(?i)B|C', 'C'
t 'a(?i)B|C', 'ac'
t '(?i)[^a]', 'A'
t '(?i)[k]', 'K'
t '(?i)[A-Z]+', 'xyz'
t 'a b c', 'abc', X
t 'a\ b', 'a b', X
t 'a # comment
b', 'ab', X
t '[ ]', ' ', X
t '(?x) a b', 'ab'
t '(?x: a b ) c', 'ab c'
t 'a(?#comment)b', 'ab'
t 'a.b', "a\nb", M
t '(?m:.)', "\n"
t '(?m).(?-m).', "\n\n"
t 'a+ ?', 'aa', X
t 'a{2} ', 'aa', X
# \K \R \X
t 'foo\Kbar', 'foobar'
t '(foo)\Kbar', 'foobar'
t 'a\Kb|c', 'ac'
t '\R', "a\r\nb"
t '\R', "a\nb"
t '\R+', "\r\n\n\r"
t '\X', "éx"
t '\X\X', "\r\nab"
# search offsets
t 'a', 'aaa', 0, 1
t 'a', 'aaa', 0, 3
t '', 'abc', 0, 3
t '$', 'abc', 0, 2
t 'b', 'abcb', 0, 2
t '\Ga', 'aab', 0, 1
t 'あ', 'いあいあ', 0, 3
t 'x*', 'abc', 0, 2
t '(?<=a)b', 'ab', 0, 1
t '\bb', 'ab', 0, 1
t 'ab', 'xxab', 0, 2, 'm'
t 'ab', 'xxab', 0, 1, 'm'
t 'a*', 'baa', 0, 1, 'm'
t '(a)|b', 'xb', 0, 1, 'm'
t '^a', 'ba', 0, 1, 'm'
t '\Ga', 'ba', 0, 1, 'm'
t '', 'abc', 0, 3, 'm'
# misc tricky
t '(a|b)*?c', 'abac'
t '(a+|b+)*c', 'aabbc'
t '((a)|(b))*', 'ab'
t '((a)|(b))*', 'ba'
t '(a)?(?(1)b|c)', 'ab' rescue nil
t '^(\w+)@(\w+)\.com$', 'user@example.com'
t '(\d+)-(\d+)', 'x 10-20 y'
t '\A\s*\z', "  \n "
t '(?:(?:a)*)*b', 'aab'
t '(?:a*)*b', 'aac'
t '(?:a|b)+?c', 'abc'
t '([a-c])x\1', 'axa bxb'
t 'x(?=y)y', 'xy'
t '[[:alpha:]&&[^a-c]]+', 'abcdef'
t 'a\Z', "ba\n", 0, 0, 'm' rescue nil
t '(?<a>x)(y)', 'xy'
t '(x)(?<a>y)', 'xy'
t '.+?(?=\.)', 'foo.bar'
t '\$\d+\.\d{2}', 'price: $10.99'
t '[0-9]+(?:\.[0-9]+)?', 'v 3.14 x'
t '(\w+)=(\w*)', 'a= b=c'
t 'a|b|', 'c'
t '(?:)', 'a'
t '()', 'a'
t '(a)?', 'b'
t '(a){0}', 'a'
t '(a){0}b\1', 'b' rescue nil
t '\p{Greek}+', 'aαβγb'
t '\p{Cyrillic}+', 'aжзb'
t '[\p{Han}\p{Hiragana}]+', 'ABC漢字ひらがなカ'
t '\w+', 'aé'
t '[\s\S]+', "a\nb"
t '[^\n]+', "ab\ncd"
t '(?i)ǅ', 'ǆ'
t '(?i)straße', 'STRASSE'
t '(?i)ß', 'ẞ'
t '^\p{Upper}\p{Lower}+$', 'Émile'
t 'é', 'é'
t "caf\\u00e9|café", "café"

# regressions found by differential fuzzing (fuzz.rb)
t '[^0-9]*?.', "\na"
t '\n*?.', "\na"
t '[^0-9]*?.+?[[:alpha:]]\X{2}', "\nÉÉ\r\nア1b"
t '(?=b).*', 'ab'
t '(?=b).*', 'ab', M
t '(?=(b)).*+', 'axb', M
t '\b.*x', '-ax', M
t '$\b(?<!a){2,3}.*', 'aA', M
t '(.*)\1x', 'zaax'
t '(?=ab\K)', 'abc'
t 'a\Kb|ac', 'ac'
t '(?(1)a|b)(c)', 'bc'
t '(a)?(?(1)b|c)', 'ab'
t '(a)?(?(1)b|c)', 'c'
t '(?<x>a)?(?(<x>)b|c)', 'c'
t '(a)(?(1)|c)', 'ac'
t '(?(1)a|b|c)', 'a'
t '(?<?>a)', 'a'
t '(?<a-b>a)', 'a'
t '(?<-a>a)', 'a'
t '\k<a>(?<a>x)', 'xx'
t '(?<a>\k<a>x)', 'xx'
t '(?(<a>)a)(?<a>b)', 'ab'
t '(A\1*?)\z', 'AA'
t '(A\1*?)\z', 'AAA'
t '(A\1*)', 'AAAA'
t '(A\1)', 'AA'
t '(|(\1.))(a)', 'Ba'
t '(\1.)', 'Ba'
t '[^a]((\1c|))*', 'Bc'
t '((\1)|(((\s))))*', "B\nbbAB", 0, 1
t '((\1)*)*', 'B', 0, 1
t '(([b])*((?!\1)))(?!c)', 'bc'
t '(?:()*.)*', 'bbb'
t '(?:b()*)*', 'bbb'
t '(?:()|a)*$', 'a'
t '(?:()|a)*$', 'aa'
t 'B(?:()|c)*$', 'Bc'
t '([^a]{0,}?)+\Zb?', 'bbaxac', 0, 5
t '(?=(\w)+)*', 'bAacx'
t '(()*\w)*', 'BbcBcxaxA'
t '\z()\1\B', 'abb'
t '()((\1\b))', 'aab', I
t '\1*?((?:.))(?!b)', 'aab'
t '(?!(\w*(?!a)*)*C)\Z', 'cBbaBxBAx'
t 'ss', 'ß', I
t 'ß', 'ss', I
t 'ss', 'xSSy', I

t '\u{61 62}+', 'abbb'
t '[\u{61 62}]+', 'abba'
t '\x{3042}', 'あ'
t '(((([b])*)+).)*', 'aaxc'
t '((.(\p{L}*)+))+', 'c11あa1'
t '(([b](([ab]*?))+)+)', 'bbc'
t '(x(((?!\s)))+)+', 'aabaABcxx'
t '(((()+.){3}))', 'BbbABcxB'

puts C.map { |pat, opts, subj, start, mode|
  re = begin
    Regexp.new(mode == "m" ? "\\G(?:#{pat})" : pat, opts)
  rescue RegexpError => e
    next [pat.b.unpack1("H*"), opts, subj.b.unpack1("H*"), start, mode, "E:" + e.message.sub(/: \/.*\z/m, "")].join("\t")
  end
  subj2 = subj.dup.force_encoding(subj.valid_encoding? ? "UTF-8" : "BINARY")
  m = subj2.byteindex(re, start) && $~
  exp = m ? (0...m.size).map { |i| o = m.byteoffset(i); o[0] ? o.join(",") : "-" }.join(";") : "N"
  names = (re.named_captures.map { |n, v| "#{n}=#{v.join(",")}" }.join(" ") rescue "")
  row = [pat.b.unpack1("H*"), opts, subj.b.unpack1("H*"), start, mode, exp]
  row << names unless names.empty?
  row.join("\t")
}
