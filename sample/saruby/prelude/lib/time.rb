# Time.parse / iso8601 / rfc2822 / httpdate / strptime without the date extension.
class Time
  MonthValue = { "JAN" => 1, "FEB" => 2, "MAR" => 3, "APR" => 4, "MAY" => 5, "JUN" => 6,
                 "JUL" => 7, "AUG" => 8, "SEP" => 9, "OCT" => 10, "NOV" => 11, "DEC" => 12 }
  RFC2822_DAY_NAME = %w[Sun Mon Tue Wed Thu Fri Sat]
  RFC2822_MONTH_NAME = %w[Jan Feb Mar Apr May Jun Jul Aug Sep Oct Nov Dec]
  ZoneOffset = { "UTC" => 0, "Z" => 0, "UT" => 0, "GMT" => 0, "EST" => -5, "EDT" => -4, "CST" => -6, "CDT" => -5,
                 "MST" => -7, "MDT" => -6, "PST" => -8, "PDT" => -7, "JST" => 9, "CET" => 1, "CEST" => 2, "BST" => 1, "IST" => 5.5 }

  class << self
    def zone_offset(zone, year = now.year)
      zone = zone.to_s.upcase
      if /\A([+-])(\d\d):?(\d\d)(?::?(\d\d))?\z/ =~ zone
        off = $2.to_i * 3600 + $3.to_i * 60 + $4.to_i
        $1 == "-" ? -off : off
      elsif ZoneOffset.key?(zone)
        (ZoneOffset[zone] * 3600).to_i
      end
    end

    def __make(year, mon, day, hour, min, sec, frac, zone, now)
      sec = sec.to_i + (frac ? Rational(frac.to_i, 10**frac.size) : 0)
      if zone
        return utc(year, mon, day, hour, min, sec) if zone == "Z" || zone == "z" || zone == "-00:00" || zone == "-0000"
        off = zone_offset(zone)
        return new(year, mon, day, hour, min, sec, off) if off
      end
      local(year, mon, day, hour, min, sec)
    end

    def parse(date, now = self.now)
      str = String(date).dup
      year = mon = day = hour = min = sec = frac = zone = nil
      if str.sub!(/(\d{1,2}):(\d\d)(?::(\d\d)(?:[.,](\d+))?)?\s*([ap]m\b)?/i, " ")
        hour, min, sec, frac = $1.to_i, $2.to_i, $3, $4
        if $5
          hour %= 12
          hour += 12 if $5.downcase == "pm"
        end
        if str.sub!(/\s*(Z\b|[A-Z]{2,5}\b(?<![AP]M)|[+-]\d\d(?::?\d\d)?(?::?\d\d)?)\s*\z/i, " ") || str.sub!(/\A\s*\S*?\s+(UTC|GMT|[+-]\d\d:?\d\d)\b/i, " ")
          zone = $1
        end
      end
      if str.sub!(/(-?\d{4,})-(\d\d?)-(\d\d?)/, " ") || str.sub!(%r{(\d{4,})/(\d\d?)/(\d\d?)}, " ")
        year, mon, day = $1.to_i, $2.to_i, $3.to_i
      elsif str.sub!(/\b(\d{4})(\d\d)(\d\d)(?:T?(\d\d)(\d\d)(\d\d)?)?\b/, " ")
        year, mon, day = $1.to_i, $2.to_i, $3.to_i
        hour, min, sec = $4.to_i, $5.to_i, $6 if $4 && !hour
      elsif str.sub!(/\b(\d\d?)(?:st|nd|rd|th)?[\s-]+(jan|feb|mar|apr|may|jun|jul|aug|sep|oct|nov|dec)[a-z]*\.?(?:[\s,-]+(-?\d+))?/i, " ")
        day, mon, year = $1.to_i, MonthValue[$2.upcase], $3&.to_i
      elsif str.sub!(/\b(jan|feb|mar|apr|may|jun|jul|aug|sep|oct|nov|dec)[a-z]*\.?\s+(\d\d?)(?:st|nd|rd|th)?\b(?:[\s,]+(-?\d+))?/i, " ")
        mon, day, year = MonthValue[$1.upcase], $2.to_i, $3&.to_i
      elsif str.sub!(%r{\b(\d\d?)/(\d\d?)(?:/(\d+))?}, " ")
        mon, day, year = $1.to_i, $2.to_i, $3&.to_i
      end
      year += (year < 69 ? 2000 : 1900) if year && year >= 0 && year < 100 && date !~ /\b\d{4}\b/
      raise ArgumentError, "no time information in #{date.inspect}" unless year || mon || day || hour
      if block_given? && year
        year = yield(year)
      end
      had_date = year || mon || day
      year ||= now.year
      mon ||= had_date ? 1 : now.mon
      day ||= had_date ? 1 : now.day
      raise ArgumentError, "argument out of range" if mon > 12 || mon < 1 || day > 31 || day < 1 || (hour || 0) > 24 || (min || 0) > 59 || sec.to_i > 60
      __make(year, mon, day, hour || 0, min || 0, sec, frac, zone, now)
    end

    def iso8601(time)
      if /\A\s*(-?\d+)-(\d\d)-(\d\d)T(\d\d):(\d\d):(\d\d)(?:[.,](\d+))?(Z|[+-]\d\d(?::?\d\d)?)?\s*\z/ix =~ time
        f = [$1.to_i, $2.to_i, $3.to_i, $4.to_i, $5.to_i, $6, $7]
        zone = $8
        zone += "00" if zone && zone.size == 3
        __make(*f, zone, nil)
      elsif /\A\s*(-?\d+)-(\d\d)-(\d\d)\s*\z/ =~ time
        local($1.to_i, $2.to_i, $3.to_i)
      else
        raise ArgumentError, "invalid xmlschema format: #{time.inspect}"
      end
    end
    alias xmlschema iso8601

    def rfc2822(date)
      if /\A\s*(?:(?:Mon|Tue|Wed|Thu|Fri|Sat|Sun)\s*,\s*)?(\d{1,2})\s+(Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\s+(\d{2,})\s+(\d{2})\s*:\s*(\d{2})(?:\s*:\s*(\d\d))?\s+([+-]\d{4}|[A-Z]+)/i =~ date
        year = $3.to_i
        year += (year < 50 ? 2000 : 1900) if $3.size < 4
        zone = $7
        return utc(year, MonthValue[$2.upcase], $1.to_i, $4.to_i, $5.to_i, $6.to_i) if zone == "-0000"
        __make(year, MonthValue[$2.upcase], $1.to_i, $4.to_i, $5.to_i, $6, nil, zone, nil)
      else
        raise ArgumentError, "not RFC 2822 compliant date: #{date.inspect}"
      end
    end
    alias rfc822 rfc2822

    def httpdate(date)
      if /\A\s*(?:Mon|Tue|Wed|Thu|Fri|Sat|Sun),\s+(\d{2})\s+(Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\s+(\d{4})\s+(\d{2}):(\d{2}):(\d{2})\s+GMT\s*\z/i =~ date
        utc($3.to_i, MonthValue[$2.upcase], $1.to_i, $4.to_i, $5.to_i, $6.to_i)
      elsif /\A\s*(?:Monday|Tuesday|Wednesday|Thursday|Friday|Saturday|Sunday),\s+(\d\d)-(Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)-(\d\d)\s+(\d\d):(\d\d):(\d\d)\s+GMT\s*\z/i =~ date
        y = $3.to_i
        utc(y + (y < 50 ? 2000 : 1900), MonthValue[$2.upcase], $1.to_i, $4.to_i, $5.to_i, $6.to_i)
      elsif /\A\s*(?:Mon|Tue|Wed|Thu|Fri|Sat|Sun)\s+(Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\s+(\d\d?)\s+(\d\d):(\d\d):(\d\d)\s+(\d{4})\s*\z/i =~ date
        utc($6.to_i, MonthValue[$1.upcase], $2.to_i, $3.to_i, $4.to_i, $5.to_i)
      else
        raise ArgumentError, "not RFC 2616 compliant date: #{date.inspect}"
      end
    end

    STRPTIME = {
      "Y" => ['([+-]?\d+)', :year], "C" => ['(\d\d)', :cent], "y" => ['(\d\d)', :yy], "m" => ['(\d\d?)', :mon], "d" => ['\s*(\d\d?)', :day],
      "e" => ['\s*(\d\d?)', :day], "j" => ['(\d{1,3})', :yday], "H" => ['\s*(\d\d?)', :hour], "k" => ['\s*(\d\d?)', :hour], "I" => ['\s*(\d\d?)', :hour12],
      "l" => ['\s*(\d\d?)', :hour12], "M" => ['(\d\d?)', :min], "S" => ['(\d\d?)', :sec], "L" => ['(\d{1,3})', :msec], "N" => ['(\d+)', :frac],
      "z" => ['(Z|[+-]\d\d(?::?\d\d)?(?::?\d\d)?|[A-Za-z]+)', :zone], "Z" => ['([A-Za-z]+|[+-]\d\d:?\d\d)', :zone], "s" => ['([+-]?\d+)', :epoch],
      "b" => ['([A-Za-z]{3,})', :monname], "B" => ['([A-Za-z]{3,})', :monname], "h" => ['([A-Za-z]{3,})', :monname], "a" => ['([A-Za-z]{3,})', nil],
      "A" => ['([A-Za-z]{3,})', nil], "p" => ['([AaPp]\.?[Mm]\.?)', :ampm], "P" => ['([AaPp]\.?[Mm]\.?)', :ampm], "u" => ['(\d)', nil], "w" => ['(\d)', nil],
    }
    STRPTIME_EXPAND = { "T" => "%H:%M:%S", "X" => "%H:%M:%S", "R" => "%H:%M", "r" => "%I:%M:%S %p", "F" => "%Y-%m-%d", "D" => "%m/%d/%y", "x" => "%m/%d/%y",
                        "c" => "%a %b %e %H:%M:%S %Y", "v" => "%e-%b-%Y", "+" => "%a %b %e %H:%M:%S %Z %Y" }

    def strptime(date, format, now = self.now)
      fmt = format.gsub(/%([TXRrFDxcv+])/) { STRPTIME_EXPAND[$1] }
      keys = []
      re = fmt.gsub(/%[-_0^#]?(?:(\d+)|:+)?([A-Za-z%])|(\s+)|([^%\s]+)/) do
        if $3 then '\s*'
        elsif $4 then Regexp.escape($4)
        elsif $2 == "%" then "%"
        elsif $2 == "n" || $2 == "t" then '\s+'
        elsif (ent = STRPTIME[$2])
          keys << ent[1]
          ent[0]
        else
          raise ArgumentError, "invalid date or strptime format - '#{date}' '#{format}'"
        end
      end
      m = Regexp.new('\A\s*' + re).match(date) or raise ArgumentError, "invalid date or strptime format - '#{date}' '#{format}'"
      h = {}
      keys.each_with_index { |k, i| h[k] = m[i + 1] if k }
      if h[:epoch]
        t = at(h[:epoch].to_i)
        return h[:zone] ? t.localtime(zone_offset(h[:zone]) || 0) : t
      end
      year = h[:year]&.to_i
      if !year && h[:yy]
        yy = h[:yy].to_i
        year = (h[:cent] ? h[:cent].to_i * 100 : (yy < 69 ? 2000 : 1900)) + yy
      end
      mon = h[:mon]&.to_i || (h[:monname] && (MonthValue[h[:monname][0, 3].upcase] or raise ArgumentError, "invalid date or strptime format - '#{date}' '#{format}'"))
      hour = (h[:hour] || h[:hour12])&.to_i
      if h[:ampm] && hour
        hour %= 12
        hour += 12 if h[:ampm] =~ /p/i
      end
      frac = h[:frac] || (h[:msec] && h[:msec].rjust(3, "0"))
      had = year || mon || h[:day] || h[:yday]
      year ||= now.year
      if h[:yday] && !mon
        t = __make(year, 1, 1, hour || 0, h[:min].to_i, h[:sec], frac, h[:zone], now)
        return t + (h[:yday].to_i - 1) * 86400
      end
      mon ||= had ? 1 : now.mon
      day = h[:day]&.to_i || (had ? 1 : now.day)
      __make(year, mon, day, hour || 0, h[:min].to_i, h[:sec], frac, h[:zone], now)
    end
  end

  def iso8601(fraction_digits = 0)
    s = strftime("%Y-%m-%dT%H:%M:%S")
    s << "." << ("%09d" % nsec)[0, fraction_digits].ljust(fraction_digits, "0") if fraction_digits > 0
    s << (utc? ? "Z" : strftime("%:z"))
  end
  alias xmlschema iso8601

  def rfc2822
    strftime("%a, %d %b %Y %H:%M:%S ") << (utc? ? "-0000" : strftime("%z"))
  end
  alias rfc822 rfc2822

  def httpdate
    getutc.strftime("%a, %d %b %Y %H:%M:%S GMT")
  end
end
