# Shared helper: emit TSV rows for [pat, subj, opts, start, mode] (used by ad-hoc probes).
def row(pat, subj, opts = 0, start = 0, mode = "s")
  re = begin
    Regexp.new(mode == "m" ? "\\G(?:#{pat})" : pat, opts)
  rescue RegexpError => e
    return [pat.b.unpack1("H*"), opts, subj.b.unpack1("H*"), start, mode, "E:" + e.message.sub(/: \/.*\z/m, "")].join("\t")
  end
  m = subj.byteindex(re, start) && $~
  exp = m ? (0...m.size).map { |i| o = m.byteoffset(i); o[0] ? o.join(",") : "-" }.join(";") : "N"
  names = re.named_captures.map { |n, v| "#{n}=#{v.join(",")}" }.join(" ")
  r = [pat.b.unpack1("H*"), opts, subj.b.unpack1("H*"), start, mode, exp]
  r << names unless names.empty?
  r.join("\t")
end
