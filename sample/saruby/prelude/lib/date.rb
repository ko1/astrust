# frozen_string_literal: true
# Date / DateTime for saruby, in Ruby: a port of CRuby's ext/date (date_core.c, date_parse.c,
# date_strptime.c, date_strftime.c and lib/date.rb).
# Original: Copyright Tadayoshi Funaba 1998-2011, date_strftime.c based on Arnold Robbins'
# public-domain strftime; distributed under the BSD-2-Clause license / Ruby's license.
#
# State of an instance (CRuby's DateData): @jd (UTC chronological Julian day), @df (UTC seconds
# in the day), @sf (nanoseconds, Integer or Rational), @of (offset seconds), @sg (start, Float),
# @cx (complex: has a time part). Civil fields are derived on demand from the local jd.

class Date
  include Comparable

  class Error < ArgumentError; end

  VERSION = "3.6.0" # :nodoc:

  ITALY = 2299161
  ENGLAND = 2361222
  JULIAN = Float::INFINITY
  GREGORIAN = -Float::INFINITY

  names = ->(a) { a.map { |s| s && s.dup.force_encoding(Encoding::US_ASCII).freeze }.freeze }
  MONTHNAMES = names.(%w[January February March April May June July August September October November December].unshift(nil))
  ABBR_MONTHNAMES = names.(%w[Jan Feb Mar Apr May Jun Jul Aug Sep Oct Nov Dec].unshift(nil))
  DAYNAMES = names.(%w[Sunday Monday Tuesday Wednesday Thursday Friday Saturday])
  ABBR_DAYNAMES = names.(%w[Sun Mon Tue Wed Thu Fri Sat])

  # Calendar arithmetic of date_core.c on unbounded Integers (no nth/period split needed).
  module Calc # :nodoc:
    DAY = 86400
    NS = 1_000_000_000
    DAY_NS = DAY * NS
    REFORM_BEGIN_JD = 2298874
    REFORM_END_JD = 2426355
    UNIX_EPOCH = 2440588
    CM_PERIOD = 213447717
    MONTHTAB = [[0, 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31],
                [0, 31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]].freeze
    YEARTAB = [[0, 0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334],
               [0, 0, 31, 60, 91, 121, 152, 182, 213, 244, 274, 305, 335]].freeze

    module_function

    def julian_leap?(y) = y % 4 == 0
    def gregorian_leap?(y) = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0

    def civil_to_jd(y, m, d, sg)
      if m <= 2
        y -= 1
        m += 12
      end
      a = y.div(100)
      b = 2 - a + a.div(4)
      jd = (1461 * (y + 4716)).div(4) + (306001 * (m + 1)).div(10000) + d + b - 1524
      jd -= b if jd < sg
      jd
    end

    def jd_to_civil(jd, sg)
      if jd < sg
        a = jd
      else
        x = (4 * jd - 7468865).div(146097)
        a = jd + 1 + x - x.div(4)
      end
      b = a + 1524
      c = (100 * b - 12210).div(36525)
      d = (1461 * c).div(4)
      e = ((b - d) * 10000).div(306001)
      dom = b - d - (306001 * e).div(10000)
      if e <= 13
        [c - 4716, e - 1, dom]
      else
        [c - 4715, e - 13, dom]
      end
    end

    def valid_civil(y, m, d, sg)
      m += 13 if m < 0
      return nil if m < 1 || m > 12
      if d < 0
        ljd = find_ldom(y, m, sg) or return nil
        ry, rm, rd = jd_to_civil(ljd + d + 1, sg)
        return nil if ry != y || rm != m
        d = rd
      end
      jd = civil_to_jd(y, m, d, sg)
      ry, rm, rd = jd_to_civil(jd, sg)
      return nil if ry != y || rm != m || rd != d
      jd
    end

    def valid_gregorian?(y, m, d)
      m += 13 if m < 0
      return false if m < 1 || m > 12
      last = MONTHTAB[gregorian_leap?(y) ? 1 : 0][m]
      d = last + d + 1 if d < 0
      d >= 1 && d <= last
    end

    def find_fdoy(y, sg)
      1.upto(30) { |d| jd = valid_civil(y, 1, d, sg) and return jd }
      nil
    end

    def find_ldoy(y, sg)
      31.downto(2) { |d| jd = valid_civil(y, 12, d, sg) and return jd }
      nil
    end

    def find_ldom(y, m, sg)
      31.downto(2) { |d| jd = valid_civil(y, m, d, sg) and return jd }
      nil
    end

    def ordinal_to_jd(y, d, sg) = find_fdoy(y, sg) + d - 1

    def jd_to_ordinal(jd, sg)
      y = jd_to_civil(jd, sg)[0]
      [y, jd - find_fdoy(y, sg) + 1]
    end

    def valid_ordinal(y, d, sg)
      if d < 0
        ljd = find_ldoy(y, sg) or return nil
        ry, rd = jd_to_ordinal(ljd + d + 1, sg)
        return nil if ry != y
        d = rd
      end
      jd = ordinal_to_jd(y, d, sg)
      ry, rd = jd_to_ordinal(jd, sg)
      return nil if ry != y || rd != d
      jd
    end

    def commercial_to_jd(y, w, d, sg)
      rjd2 = find_fdoy(y, sg) + 3
      (rjd2 - (rjd2 % 7)) + 7 * (w - 1) + (d - 1)
    end

    def jd_to_commercial(jd, sg)
      a = jd_to_civil(jd - 3, sg)[0]
      j = commercial_to_jd(a + 1, 1, 1, sg)
      if jd >= j
        y = a + 1
      else
        j = commercial_to_jd(a, 1, 1, sg)
        y = a
      end
      w = 1 + (jd - j).div(7)
      d = (jd + 1) % 7
      d = 7 if d == 0
      [y, w, d]
    end

    def valid_commercial(y, w, d, sg)
      d += 8 if d < 0
      if w < 0
        return nil if w < -53
        rjd2 = commercial_to_jd(y + 1, 1, 1, sg)
        ry, rw, = jd_to_commercial(rjd2 + w * 7, sg)
        return nil if ry != y
        w = rw
      elsif w < 1 || w > 53
        return nil
      end
      jd = commercial_to_jd(y, w, d, sg)
      ry, rw, rd = jd_to_commercial(jd, sg)
      return nil if y != ry || w != rw || d != rd
      jd
    end

    def weeknum_to_jd(y, w, d, f, sg)
      rjd2 = find_fdoy(y, sg) + 6
      (rjd2 - ((rjd2 - f + 1) % 7) - 7) + 7 * w + d
    end

    def jd_to_weeknum(jd, f, sg)
      y = jd_to_civil(jd, sg)[0]
      rjd = find_fdoy(y, sg) + 6
      j = jd - (rjd - ((rjd - f) + 1) % 7) + 7
      [y, j.div(7), j % 7]
    end

    def valid_weeknum(y, w, d, f, sg)
      d += 7 if d < 0
      if w < 0
        rjd2 = weeknum_to_jd(y + 1, 1, f, f, sg)
        ry, rw, = jd_to_weeknum(rjd2 + w * 7, f, sg)
        return nil if ry != y
        w = rw
      end
      jd = weeknum_to_jd(y, w, d, f, sg)
      ry, rw, rd = jd_to_weeknum(jd, f, sg)
      return nil if y != ry || w != rw || d != rd
      jd
    end

    def valid_time(h, min, s)
      h += 24 if h < 0
      min += 60 if min < 0
      s += 60 if s < 0
      return nil if h < 0 || h > 24 || min < 0 || min > 59 || s < 0 || s > 59 || (h == 24 && (min > 0 || s > 0))
      [h, min, s]
    end

    def valid_start?(sg)
      return false if sg.nan?
      return true if sg.infinite?
      !(sg < REFORM_BEGIN_JD || sg > REFORM_END_JD)
    end

    def numeric!(x, field)
      raise TypeError, "invalid #{field} (not numeric)" unless x.is_a?(Numeric)
    end

    def expect_numeric!(x)
      raise TypeError, "expected numeric" unless x.is_a?(Numeric)
    end

    def num2dbl(v)
      case v
      when Float then v
      when Integer, Rational then v.to_f
      when nil then raise TypeError, "can't convert nil into Float"
      when Numeric then v.to_f
      else raise TypeError, "no implicit conversion to float from #{v.class.name.downcase}"
      end
    end

    def num2int(v)
      case v
      when Integer then v
      when Float then v.truncate
      when nil then raise TypeError, "no implicit conversion from nil to integer"
      else v.to_int
      end
    end

    def val2sg(v)
      sg = num2dbl(v)
      unless valid_start?(sg)
        warn("invalid start is ignored", uplevel: 2) if $VERBOSE
        sg = ITALY.to_f
      end
      sg
    end

    def wholenum?(x)
      case x
      when Integer then true
      when Float then x.round == x
      when Rational then x.denominator == 1
      else false
      end
    end

    # [integer part, fraction / per] (d_trunc, h_trunc, min_trunc, s_trunc)
    def trunc(x, per = nil)
      return [x.is_a?(Integer) ? x : x.to_i, 0] if wholenum?(x)
      fr = x % 1
      fr = fr.quo(per) if per
      [x.div(1), fr]
    end

    def canon(x) = x.is_a?(Rational) && x.denominator == 1 ? x.numerator : x

    def offset_to_sec(vof)
      try_rational = true
      loop do
        case vof
        when Integer
          return nil unless vof == -1 || vof == 0 || vof == 1
          return vof * DAY
        when Float
          n = vof * DAY
          return nil if n < -DAY || n > DAY
          r = n.round
          warn("fraction of offset is ignored", uplevel: 3) if $VERBOSE && r != n
          return r
        when Rational
          vs = vof * DAY
          if vs.is_a?(Rational) && vs.denominator != 1
            vn = vs.round
            warn("fraction of offset is ignored", uplevel: 3) if $VERBOSE && vn != vs
          else
            vn = vs.to_i
          end
          return nil if vn < -DAY || vn > DAY
          return vn
        when String
          vs = Date.__send__(:__zone_to_diff, vof)
          return nil unless vs.is_a?(Integer) && vs >= -DAY && vs <= DAY
          return vs
        else
          expect_numeric!(vof)
          vof = vof.to_r
          unless vof.is_a?(Rational)
            raise TypeError, "wrong argument type #{vof.class} (expected Rational)" unless try_rational
            try_rational = false
          end
        end
      end
    end

    def val2off(vof)
      of = offset_to_sec(vof)
      unless of
        warn("invalid offset is ignored", uplevel: 2) if $VERBOSE
        of = 0
      end
      of
    end

    def us(s) = s.force_encoding(Encoding::US_ASCII)
  end
  private_constant :Calc

  # ---- construction ----

  def __set(jd, df, sf, of, sg, cx) # :nodoc:
    @jd = jd
    @df = df
    @sf = Calc.canon(sf)
    @of = of
    @sg = sg.to_f
    @cx = cx
    ljd, ldf = __local
    @civil = Calc.jd_to_civil(ljd, @sg)
    @time = [ldf / 3600, ldf % 3600 / 60, ldf % 60]
    self
  end
  private :__set

  def self.__new_raw(jd, df, sf, of, sg, cx) # :nodoc:
    allocate.__send__(:__set, jd, df, sf, of, sg, cx)
  end
  private_class_method :__new_raw

  # local civil time (h, min, s) -> UTC [jd, df]
  def self.__local_to_utc(ljd, ldf, of) # :nodoc:
    df = ldf - of
    if df < 0
      [ljd - 1, df + Calc::DAY]
    elsif df >= Calc::DAY
      [ljd + 1, df - Calc::DAY]
    else
      [ljd, df]
    end
  end
  private_class_method :__local_to_utc

  def initialize(*args)
    __civil_init(args)
    nil
  end
  private :initialize

  def __civil_init(args) # :nodoc:
    raise ArgumentError, "wrong number of arguments (given #{args.size}, expected 0..4)" if args.size > 4
    y = -4712
    m = 1
    d = 1
    fr2 = 0
    sg = ITALY
    n = args.size
    sg = Calc.val2sg(args[3]) if n >= 4
    if n >= 3
      Calc.numeric!(args[2], "day")
      d, fr = Calc.trunc(args[2])
      d = Calc.num2int(d)
      fr2 = fr unless fr == 0
    end
    if n >= 2
      Calc.numeric!(args[1], "month")
      m = Calc.num2int(args[1])
    end
    if n >= 1
      Calc.numeric!(args[0], "year")
      y = args[0]
      y = y.to_i unless y.is_a?(Integer)
    end
    jd = Calc.valid_civil(y, m, d, sg) or raise Error, "invalid date"
    __set(jd, 0, 0, 0, sg, false)
    fr2 == 0 ? self : self + fr2
  end
  private :__civil_init

  class << self
    def civil(*args) = allocate.__send__(:__civil_init, args)

    def jd(*args)
      raise ArgumentError, "wrong number of arguments (given #{args.size}, expected 0..2)" if args.size > 2
      jd = 0
      fr2 = 0
      sg = ITALY
      sg = Calc.val2sg(args[1]) if args.size >= 2
      if args.size >= 1
        Calc.numeric!(args[0], "jd")
        jd, fr2 = Calc.trunc(args[0])
      end
      ret = __new_raw(jd, 0, 0, 0, sg, false)
      fr2 == 0 ? ret : ret + fr2
    end

    def ordinal(*args)
      raise ArgumentError, "wrong number of arguments (given #{args.size}, expected 0..3)" if args.size > 3
      y = -4712
      d = 1
      fr2 = 0
      sg = ITALY
      sg = Calc.val2sg(args[2]) if args.size >= 3
      if args.size >= 2
        Calc.numeric!(args[1], "yday")
        d, fr2 = Calc.trunc(args[1])
        d = Calc.num2int(d)
      end
      if args.size >= 1
        Calc.numeric!(args[0], "year")
        y = Calc.num2int(args[0])
      end
      jd = Calc.valid_ordinal(y, d, sg) or raise Error, "invalid date"
      ret = __new_raw(jd, 0, 0, 0, sg, false)
      fr2 == 0 ? ret : ret + fr2
    end

    def commercial(*args)
      raise ArgumentError, "wrong number of arguments (given #{args.size}, expected 0..4)" if args.size > 4
      y = -4712
      w = 1
      d = 1
      fr2 = 0
      sg = ITALY
      sg = Calc.val2sg(args[3]) if args.size >= 4
      if args.size >= 3
        Calc.numeric!(args[2], "cwday")
        d, fr2 = Calc.trunc(args[2])
        d = Calc.num2int(d)
      end
      if args.size >= 2
        Calc.numeric!(args[1], "cweek")
        w = Calc.num2int(args[1])
      end
      if args.size >= 1
        Calc.numeric!(args[0], "year")
        y = Calc.num2int(args[0])
      end
      jd = Calc.valid_commercial(y, w, d, sg) or raise Error, "invalid date"
      ret = __new_raw(jd, 0, 0, 0, sg, false)
      fr2 == 0 ? ret : ret + fr2
    end

    def today(sg = ITALY)
      sg = Calc.val2sg(sg)
      t = Time.at(*Process.clock_gettime(Process::CLOCK_REALTIME, :nanosecond).divmod(Calc::NS), :nsec)
      __new_raw(Calc.civil_to_jd(t.year, t.mon, t.mday, -Float::INFINITY), 0, 0, 0, sg, false)
    end

    def valid_jd?(jd, sg = ITALY)
      return false unless jd.is_a?(Numeric)
      Calc.num2dbl(sg)
      true
    end

    def valid_civil?(y, m, d, sg = ITALY)
      return false unless y.is_a?(Numeric) && m.is_a?(Numeric) && d.is_a?(Numeric)
      m = Calc.num2int(m)
      d = Calc.num2int(d)
      sg = __valid_sg(sg)
      y = y.to_i unless y.is_a?(Integer)
      !Calc.valid_civil(y, m, d, sg).nil?
    end
    alias valid_date? valid_civil?

    def valid_ordinal?(y, d, sg = ITALY)
      return false unless y.is_a?(Numeric) && d.is_a?(Numeric)
      d = Calc.num2int(d)
      sg = __valid_sg(sg)
      !Calc.valid_ordinal(Calc.num2int(y), d, sg).nil?
    end

    def valid_commercial?(y, w, d, sg = ITALY)
      return false unless y.is_a?(Numeric) && w.is_a?(Numeric) && d.is_a?(Numeric)
      w = Calc.num2int(w)
      d = Calc.num2int(d)
      sg = __valid_sg(sg)
      !Calc.valid_commercial(Calc.num2int(y), w, d, sg).nil?
    end

    def julian_leap?(y)
      Calc.numeric!(y, "year")
      Calc.julian_leap?(Calc.num2int(y))
    end

    def gregorian_leap?(y)
      Calc.numeric!(y, "year")
      Calc.gregorian_leap?(Calc.num2int(y))
    end
    alias leap? gregorian_leap?

    def _load(s) # :nodoc:
      a = Marshal.load(s)
      obj = allocate
      obj.marshal_load(a)
      obj
    end

    private

    def __valid_sg(sg)
      sg = Calc.num2dbl(sg)
      unless Calc.valid_start?(sg)
        warn("invalid start is ignored", uplevel: 2) if $VERBOSE
        sg = 0.0
      end
      sg
    end
  end

  # ---- derived fields ----

  def __local # :nodoc:
    return [@jd, 0] unless @cx
    ldf = @df + @of
    if ldf < 0
      [@jd - 1, ldf + Calc::DAY]
    elsif ldf >= Calc::DAY
      [@jd + 1, ldf - Calc::DAY]
    else
      [@jd, ldf]
    end
  end
  private :__local

  def __ljd = __local[0] # :nodoc:
  private :__ljd

  def __civil = @civil # :nodoc:
  def __time = @time # :nodoc:
  def __wday = (__ljd + 1) % 7 # :nodoc:
  def __cwday = (w = __wday) == 0 ? 7 : w # :nodoc:
  private :__civil, :__time, :__wday, :__cwday

  def __sf_in_sec = @sf.is_a?(Integer) ? Rational(@sf, Calc::NS) : @sf.quo(Calc::NS) # :nodoc:
  private :__sf_in_sec

  def __ns_to_day(n) = n.is_a?(Integer) ? Rational(n, Calc::DAY_NS) : n.quo(Calc::DAY_NS) # :nodoc:
  private :__ns_to_day

  def __julian? # :nodoc:
    return @sg > 0 if @sg.infinite?
    @jd < @sg
  end
  private :__julian?

  def ajd
    unless @cx
      return Rational(@jd * 2 - 1, 2)
    end
    r = @jd
    df = @df - Calc::DAY / 2
    r += Rational(df, Calc::DAY) if df != 0
    r += __ns_to_day(@sf) if @sf != 0
    r
  end

  def amjd
    r = Rational(@jd - 2400001, 1)
    return r unless @cx
    r += Rational(@df, Calc::DAY) if @df != 0
    r += __ns_to_day(@sf) if @sf != 0
    r
  end

  def jd = __ljd
  def mjd = __ljd - 2400001
  def ld = __ljd - 2299160
  def year = __civil[0]
  def mon = __civil[1]
  alias month mon
  def mday = __civil[2]
  alias day mday

  def day_fraction
    return 0 unless @cx
    fr = Rational(__local[1], Calc::DAY)
    fr += __ns_to_day(@sf) if @sf != 0
    fr
  end

  def __yday # :nodoc:
    jd = __ljd
    y, m, d = __civil
    if (@sg.infinite? && @sg < 0) || (jd - @sg) > 366
      Calc::YEARTAB[Calc.gregorian_leap?(y) ? 1 : 0][m] + d
    elsif @sg.infinite? && @sg > 0
      Calc::YEARTAB[Calc.julian_leap?(y) ? 1 : 0][m] + d
    else
      Calc.jd_to_ordinal(jd, @sg)[1]
    end
  end

  def __cwyear = Calc.jd_to_commercial(__ljd, @sg)[0] # :nodoc:
  def __cweek = Calc.jd_to_commercial(__ljd, @sg)[1] # :nodoc:
  private :__yday, :__cwyear, :__cweek
  def yday = __yday
  def cwyear = __cwyear
  def cweek = __cweek

  def cwday = __cwday
  def wday = __wday
  def __wnum0 = Calc.jd_to_weeknum(__ljd, 0, @sg)[1] # :nodoc:
  def __wnum1 = Calc.jd_to_weeknum(__ljd, 1, @sg)[1] # :nodoc:
  private :__wnum0, :__wnum1

  def sunday? = __wday == 0
  def monday? = __wday == 1
  def tuesday? = __wday == 2
  def wednesday? = __wday == 3
  def thursday? = __wday == 4
  def friday? = __wday == 5
  def saturday? = __wday == 6

  def hour = 0
  def min = 0
  alias minute min
  def sec = 0
  alias second sec
  private :hour, :min, :minute, :sec, :second

  def julian? = __julian?
  def gregorian? = !__julian?

  def leap?
    return Calc.gregorian_leap?(__civil[0]) unless __julian?
    jd = Calc.civil_to_jd(__civil[0], 3, 1, @sg)
    Calc.jd_to_civil(jd - 1, @sg)[2] == 29
  end

  def start = @sg

  def new_start(*args)
    raise ArgumentError, "wrong number of arguments (given #{args.size}, expected 0..1)" if args.size > 1
    sg = args.empty? ? ITALY : Calc.val2sg(args[0])
    __with(sg: sg)
  end

  def italy = __with(sg: ITALY)
  def england = __with(sg: ENGLAND)
  def julian = __with(sg: JULIAN)
  def gregorian = __with(sg: GREGORIAN)

  def __with(jd: @jd, df: @df, sf: @sf, of: @of, sg: @sg, cx: @cx, klass: self.class) # :nodoc:
    klass.allocate.__send__(:__set, jd, df, sf, of, sg, cx)
  end
  private :__with

  # ---- arithmetic ----

  def +(other)
    try_rational = true
    loop do
      case other
      when Integer
        return __with(jd: @jd + other)
      when Float
        o = other
        s = o > 0 ? 1 : -1
        o = -o if s < 0
        tmp = o.truncate
        o -= tmp
        jd = tmp
        o *= Calc::DAY
        tmp = o.truncate
        df = tmp
        o -= tmp
        o *= Calc::NS
        sf = o.round
        if s < 0
          jd = -jd
          df = -df
          sf = -sf
        end
        return __plus_parts(jd, df, sf)
      when Rational
        if other.denominator == 1
          other = other.numerator
          next
        end
        s = other > 0 ? 1 : -1
        other = -other if s < 0
        jd = other.div(1)
        t = (other % 1) * Calc::DAY
        df = t.div(1)
        sf = (t % 1) * Calc::NS
        if s < 0
          jd = -jd
          df = -df
          sf = -sf
        end
        return __plus_parts(jd, df, sf)
      else
        Calc.expect_numeric!(other)
        other = other.to_r
        unless other.is_a?(Rational)
          raise TypeError, "wrong argument type #{other.class} (expected Rational)" unless try_rational
          try_rational = false
        end
      end
    end
  end

  def __plus_parts(jd, df, sf) # :nodoc:
    if sf == 0
      sf = @sf
    else
      sf = @sf + sf
      if sf < 0
        df -= 1
        sf += Calc::NS
      elsif sf >= Calc::NS
        df += 1
        sf -= Calc::NS
      end
    end
    if df == 0
      df = @df
    else
      df = @df + df
      if df < 0
        jd -= 1
        df += Calc::DAY
      elsif df >= Calc::DAY
        jd += 1
        df -= Calc::DAY
      end
    end
    jd = @jd + jd
    __with(jd: jd, df: df, sf: sf, cx: !(df == 0 && sf == 0 && @of == 0))
  end
  private :__plus_parts

  def -(other)
    if other.is_a?(Date)
      d = @jd - other.__send__(:__raw_jd)
      df = @df - other.__send__(:__raw_df)
      sf = @sf - other.__send__(:__raw_sf)
      if df < 0
        d -= 1
        df += Calc::DAY
      elsif df >= Calc::DAY
        d += 1
        df -= Calc::DAY
      end
      if sf < 0
        df -= 1
        sf += Calc::NS
      elsif sf >= Calc::NS
        df += 1
        sf -= Calc::NS
      end
      r = 0
      r += Rational(d, 1) if d != 0
      r += Rational(df, Calc::DAY) if df != 0
      r += __ns_to_day(sf) if sf != 0
      return r.is_a?(Rational) ? r : Rational(r, 1)
    end
    case other
    when Integer, Float then self + -other
    else
      Calc.expect_numeric!(other) unless other.is_a?(Rational)
      self + -other
    end
  end

  def __raw_jd = @jd # :nodoc:
  def __raw_df = @df # :nodoc:
  def __raw_sf = @sf # :nodoc:
  def __raw_of = @of # :nodoc:
  def __complex? = @cx # :nodoc:
  private :__raw_jd, :__raw_df, :__raw_sf, :__raw_of, :__complex?

  def next_day(n = 1) = self + n
  def prev_day(n = 1) = self - n
  def next = next_day
  alias succ next

  def >>(other)
    t = __civil[0] * 12 + (__civil[1] - 1) + other
    y = t.div(12)
    m = (t % 12).to_i + 1
    y = y.to_i
    d = __civil[2]
    jd = nil
    loop do
      jd = Calc.valid_civil(y, m, d, @sg) and break
      d -= 1
      raise Error, "invalid date" if d < 1
    end
    self + (jd - __ljd)
  end

  def <<(other)
    Calc.expect_numeric!(other)
    self >> -other
  end

  def next_month(n = 1) = self >> n
  def prev_month(n = 1) = self << n
  def next_year(n = 1) = self >> n * 12
  def prev_year(n = 1) = self << n * 12

  def step(limit, by = 1)
    return to_enum(:step, limit, by) unless block_given?
    date = self
    c = by <=> 0
    if c < 0
      while (date <=> limit) >= 0
        yield date
        date += by
      end
    elsif c == 0
      loop { yield date }
    else
      while (date <=> limit) <= 0
        yield date
        date += by
      end
    end
    self
  end

  def upto(max)
    return to_enum(:upto, max) unless block_given?
    date = self
    while (date <=> max) <= 0
      yield date
      date += 1
    end
    self
  end

  def downto(min)
    return to_enum(:downto, min) unless block_given?
    date = self
    while (date <=> min) >= 0
      yield date
      date += -1
    end
    self
  end

  # ---- comparison ----

  def <=>(other)
    if other.is_a?(Date)
      a = [@jd, @df, @sf]
      b = [other.__send__(:__raw_jd), other.__send__(:__raw_df), other.__send__(:__raw_sf)]
      a <=> b
    elsif other.is_a?(Numeric)
      ajd <=> other
    else
      __coerce_cmp(other, :<=>)
    end
  end

  def ===(other)
    if other.is_a?(Date)
      __ljd == (__julian? == other.__send__(:__julian?) ? other.__send__(:__ljd) : other.jd)
    elsif other.is_a?(Numeric)
      __ljd == other
    else
      __coerce_cmp(other, :==)
    end
  end

  # rb_num_coerce_cmp
  def __coerce_cmp(other, op) # :nodoc:
    return nil unless other.respond_to?(:coerce)
    ary = other.coerce(self)
    return nil if ary.nil?
    raise TypeError, "coerce must return [x, y]" unless ary.is_a?(Array) && ary.size == 2
    ary[0].__send__(op, ary[1])
  end
  private :__coerce_cmp

  def eql?(other) = other.is_a?(Date) && (self <=> other) == 0
  def hash = [@jd, @df, @sf].hash

  # ---- formatting ----

  def to_s = Calc.us(__strftime("%Y-%m-%d"))

  def inspect
    Calc.us(+"#<#{self.class}: #{to_s} ((#{@jd.inspect}j,#{@df}s,#{@sf.inspect}n),#{format('%+d', @of)}s,#{format('%.0f', @sg)}j)>")
  end

  def strftime(*args)
    raise ArgumentError, "wrong number of arguments (given #{args.size}, expected 0..1)" if args.size > 1
    __strftime_api(args, "%Y-%m-%d")
  end

  def __strftime_api(args, default) # :nodoc:
    if args.empty?
      return Calc.us(__strftime(default))
    end
    fmt = args[0]
    fmt = String.try_convert(fmt) || raise(TypeError, "no implicit conversion of #{fmt.nil? ? 'nil' : fmt.class} into String")
    raise ArgumentError, "format should have ASCII compatible encoding" unless fmt.encoding.ascii_compatible?
    __strftime(fmt).force_encoding(fmt.encoding)
  end
  private :__strftime_api

  def asctime = Calc.us(__strftime("%a %b %e %H:%M:%S %Y"))
  alias ctime asctime
  def iso8601 = Calc.us(__strftime("%Y-%m-%d"))
  alias xmlschema iso8601
  def rfc3339 = Calc.us(__strftime("%Y-%m-%dT%H:%M:%S%:z"))
  def rfc2822 = Calc.us(__strftime("%a, %-d %b %Y %T %z"))
  alias rfc822 rfc2822
  def httpdate = Calc.us(__with(of: 0, cx: true).__send__(:__strftime, "%a, %d %b %Y %T GMT"))

  def __jisx0301_fmt # :nodoc:
    d = __ljd
    return "%Y-%m-%d" if d < 2405160
    c, s = if d < 2419614 then ["M", 1867]
           elsif d < 2424875 then ["T", 1911]
           elsif d < 2447535 then ["S", 1925]
           elsif d < 2458605 then ["H", 1988]
           else ["R", 2018]
           end
    format("%s%02d.%%m.%%d", c, __civil[0] - s)
  end
  private :__jisx0301_fmt

  def jisx0301 = Calc.us(__strftime(__jisx0301_fmt))

  DECONSTRUCT_DATE_KEYS = %i[year month day yday wday].freeze # :nodoc:
  DECONSTRUCT_DT_KEYS = %i[hour min sec sec_fraction zone].freeze # :nodoc:
  private_constant :DECONSTRUCT_DATE_KEYS, :DECONSTRUCT_DT_KEYS

  def __deconstruct(keys, dt) # :nodoc:
    all = keys.nil?
    unless all || keys.is_a?(Array)
      raise TypeError, "wrong argument type #{keys.class} (expected Array or nil)"
    end
    keys = DECONSTRUCT_DATE_KEYS + (dt ? DECONSTRUCT_DT_KEYS : []) if all
    h = {}
    keys.each do |k|
      case k
      when :year then h[k] = __civil[0]
      when :month then h[k] = __civil[1]
      when :day then h[k] = __civil[2]
      when :yday then h[k] = __yday
      when :wday then h[k] = __wday
      end
      next unless dt
      case k
      when :hour then h[k] = __time[0]
      when :min then h[k] = __time[1]
      when :sec then h[k] = __time[2]
      when :sec_fraction then h[k] = __sf_in_sec
      when :zone then h[k] = __zone
      end
    end
    h
  end
  private :__deconstruct

  def deconstruct_keys(keys) = __deconstruct(keys, false)

  def __zone # :nodoc:
    return Calc.us(+"+00:00") unless @cx
    a = @of.abs
    Calc.us(format("%c%02d:%02d", @of < 0 ? "-" : "+", a / 3600, a % 3600 / 60))
  end
  private :__zone

  # ---- marshal ----

  def marshal_dump
    [@jd.div(Calc::CM_PERIOD), @jd % Calc::CM_PERIOD, @df, @sf, @of, @sg]
  end

  def marshal_load(a)
    raise FrozenError, "can't modify frozen #{self.class}: #{inspect}" if frozen?
    raise TypeError, "expected an array" unless a.is_a?(Array)
    case a.size
    when 2, 3
      if a.size == 2
        ajd = a[0] - Rational(1, 2)
        vof = 0
        vsg = a[1]
        vsg = vsg ? GREGORIAN : JULIAN unless vsg.is_a?(Numeric)
      else
        ajd, vof, vsg = a
      end
      d = ajd + Rational(1, 2)
      jd = d.floor
      f = d % 1
      s = f * Calc::DAY
      df = s.floor
      sf = (s % 1) * Calc::NS
      of = (vof * Calc::DAY).round
      sg = Calc.num2dbl(vsg)
    when 6
      nth, jd, df, sf, of, sg = a
      jd = nth * Calc::CM_PERIOD + Calc.num2int(jd)
      df = Calc.num2int(df)
      of = Calc.num2int(of)
      sg = Calc.num2dbl(sg)
    else
      raise TypeError, "invalid size"
    end
    __set(jd, df, sf, of, sg, self.is_a?(DateTime) || df != 0 || sf != 0 || of != 0)
  end

  # ---- conversions ----

  def to_date = self

  def to_datetime
    __with(df: 0, sf: 0, klass: DateTime)
  end

  def to_time
    y, m, d = (__julian? ? gregorian : self).__send__(:__civil)
    Time.local(y, m, d)
  end

  def infinite? = false
end

class Date
  # ---- strftime (date_strftime.c) ----

  DAYS_L = %w[Sunday Monday Tuesday Wednesday Thursday Friday Saturday].freeze # :nodoc:
  MONTHS_L = %w[January February March April May June July August September October November December].freeze # :nodoc:
  private_constant :DAYS_L, :MONTHS_L

  def __tmx_secs # :nodoc:
    s = (@jd - Calc::UNIX_EPOCH) * Calc::DAY
    return s unless @cx
    @df == 0 ? s : s + @df
  end
  private :__tmx_secs

  def __tmx_msecs # :nodoc:
    s = __tmx_secs * 1000
    return s unless @cx
    @sf == 0 ? s : s + @sf / 1_000_000
  end
  private :__tmx_msecs

  def __strftime(fmt) # :nodoc:
    out = (+"").b
    __strftime_into(out, fmt.b)
    out
  end
  private :__strftime

  def __fmt_num(out, def_pad, def_prec, val, precision, flags, padding) # :nodoc:
    precision = def_prec if precision <= 0
    precision = 1 if flags[:left]
    zero = padding == "0" || (padding.nil? && def_pad == "0")
    out << format(zero ? "%0*d" : "%*d", precision, val)
  end
  private :__fmt_num

  def __strftime_into(out, fmt) # :nodoc:
    i = 0
    n = fmt.bytesize
    while i < n
      c = fmt.getbyte(i)
      if c != 37 # '%'
        out << c.chr
        i += 1
        next
      end
      sp = i
      precision = -1
      flags = {}
      padding = nil
      colons = 0
      tp = nil
      handled = false
      loop do
        i += 1
        ch = i < n ? fmt.getbyte(i).chr : "\0"
        flag_found = -> { precision > 0 || flags[:locale_e] || flags[:locale_o] || flags[:colons] }
        case ch
        when "\0"
          i -= 1
          tp = :unknown
        when "A", "a"
          if flags[:chcase]
            flags.delete(:lower); flags.delete(:chcase); flags[:upper] = true
          end
          w = __wday
          tp = ch == "A" ? DAYS_L[w] : DAYS_L[w][0, 3]
        when "B", "b", "h"
          if flags[:chcase]
            flags.delete(:lower); flags.delete(:chcase); flags[:upper] = true
          end
          m = __civil[1]
          tp = ch == "B" ? MONTHS_L[m - 1] : MONTHS_L[m - 1][0, 3]
        when "C"
          __fmt_num(out, "0", 2, __civil[0].div(100), precision, flags, padding)
          handled = true
        when "c" then tp = [:sub, "%a %b %e %H:%M:%S %Y"]
        when "D", "x" then tp = [:sub, "%m/%d/%y"]
        when "d", "e"
          __fmt_num(out, ch == "d" ? "0" : " ", 2, __civil[2], precision, flags, padding)
          handled = true
        when "F" then tp = [:sub, "%Y-%m-%d"]
        when "G", "Y"
          y = ch == "G" ? __cwyear : __civil[0]
          __fmt_num(out, "0", y >= 0 ? 4 : 5, y, precision, flags, padding)
          handled = true
        when "g", "y"
          y = (ch == "g" ? __cwyear : __civil[0]) % 100
          __fmt_num(out, "0", 2, y, precision, flags, padding)
          handled = true
        when "H", "k"
          __fmt_num(out, ch == "H" ? "0" : " ", 2, __time[0], precision, flags, padding)
          handled = true
        when "I", "l"
          v = __time[0]
          v = 12 if v == 0
          v -= 12 if v > 12
          __fmt_num(out, ch == "I" ? "0" : " ", 2, v, precision, flags, padding)
          handled = true
        when "j"
          __fmt_num(out, "0", 3, __yday, precision, flags, padding)
          handled = true
        when "L", "N"
          w = ch == "L" ? 3 : 9
          precision = w if precision <= 0
          subsec = __sf_in_sec
          ww = precision
          while ww >= 9
            subsec *= 1_000_000_000
            ww -= 9
          end
          subsec *= 10**ww if ww > 0
          out << format("%0*d", precision, subsec.div(1))
          handled = true
        when "M"
          __fmt_num(out, "0", 2, __time[1], precision, flags, padding)
          handled = true
        when "m"
          __fmt_num(out, "0", 2, __civil[1], precision, flags, padding)
          handled = true
        when "n", "t"
          __fill_padding(out, 1, precision, flags, padding)
          out << (ch == "n" ? "\n" : "\t")
          handled = true
        when "P", "p"
          if (ch == "p" && flags[:chcase]) || (ch == "P" && !(flags[:chcase] || flags[:upper]))
            flags.delete(:upper); flags.delete(:chcase); flags[:lower] = true
          end
          tp = __time[0] < 12 ? "AM" : "PM"
        when "Q"
          __fmt_num(out, "0", 1, __tmx_msecs, precision, flags, padding)
          handled = true
        when "R" then tp = [:sub, "%H:%M"]
        when "r" then tp = [:sub, "%I:%M:%S %p"]
        when "S"
          __fmt_num(out, "0", 2, __time[2], precision, flags, padding)
          handled = true
        when "s"
          __fmt_num(out, "0", 1, __tmx_secs, precision, flags, padding)
          handled = true
        when "T", "X" then tp = [:sub, "%H:%M:%S"]
        when "U", "W"
          __fmt_num(out, "0", 2, ch == "U" ? __wnum0 : __wnum1, precision, flags, padding)
          handled = true
        when "u"
          __fmt_num(out, "0", 1, __cwday, precision, flags, padding)
          handled = true
        when "V"
          __fmt_num(out, "0", 2, __cweek, precision, flags, padding)
          handled = true
        when "v" then tp = [:sub, "%e-%^b-%Y"]
        when "w"
          __fmt_num(out, "0", 1, __wday, precision, flags, padding)
          handled = true
        when "Z"
          if flags[:chcase]
            flags.delete(:upper); flags.delete(:chcase); flags[:lower] = true
          end
          tp = __zone.b
        when "z"
          r = __fmt_z(out, colons, precision, flags, padding)
          if r == :unknown
            i -= 1
            tp = :unknown
          else
            handled = true
          end
        when "+" then tp = [:sub, "%a %b %e %H:%M:%S %Z %Y"]
        when "E"
          flags[:locale_e] = true
          nx = fmt.getbyte(i + 1)
          next if nx && "cCxXyY".include?(nx.chr)
          tp = :unknown
        when "O"
          flags[:locale_o] = true
          nx = fmt.getbyte(i + 1)
          next if nx && "deHkIlmMSuUVwWy".include?(nx.chr)
          tp = :unknown
        when ":"
          flags[:colons] = true
          l = 0
          l += 1 while i + l < n && fmt.getbyte(i + l) == 58
          if i + l < n && fmt.getbyte(i + l) == 122 # 'z'
            colons = l
            i += l - 1
            next
          end
          tp = :unknown
        when "_"
          if flag_found.()
            tp = :unknown
          else
            padding = " "
            next
          end
        when "-"
          if flag_found.()
            tp = :unknown
          else
            flags[:left] = true
            next
          end
        when "^"
          if flag_found.()
            tp = :unknown
          else
            flags[:upper] = true
            next
          end
        when "#"
          if flag_found.()
            tp = :unknown
          else
            flags[:chcase] = true
            next
          end
        when "0".."9"
          if ch == "0"
            if flag_found.()
              tp = :unknown
            else
              padding = "0"
            end
          end
          unless tp == :unknown
            j = i
            j += 1 while j < n && fmt.getbyte(j).between?(48, 57)
            precision = fmt.byteslice(i, j - i).to_i
            i = j - 1
            next
          end
        when "%"
          __fill_padding(out, 1, precision, flags, padding)
          out << "%"
          handled = true
        else
          tp = :unknown
        end
        break
      end
      i += 1
      next if handled
      if tp.is_a?(Array)
        sub = (+"").b
        __strftime_into(sub, tp[1].b)
        sub.upcase! if flags[:upper]
        if !flags[:left] && precision > sub.bytesize
          out << (padding || " ") * (precision - sub.bytesize)
        end
        out << sub
        next
      end
      if tp == :unknown
        tp = fmt.byteslice(sp, i - sp)
        precision = -1
        flags = {}
        padding = nil
      end
      next if tp.empty?
      __fill_padding(out, tp.bytesize, precision, flags, padding)
      s = tp.dup
      if flags[:upper]
        s = s.upcase
      elsif flags[:lower]
        s = s.downcase
      end
      out << s.b
    end
    out
  end
  private :__strftime_into

  def __fill_padding(out, i, precision, flags, padding) # :nodoc:
    if !flags[:left] && precision > i
      out << (padding || " ") * (precision - i)
    end
  end
  private :__fill_padding

  def __fmt_z(out, colons, precision, flags, padding) # :nodoc:
    off = @cx ? @of : 0
    aoff = off.abs
    hl = aoff / 3600 < 10 ? 1 : 2
    hw = 2
    hw = 1 if flags[:left] && hl == 1
    case colons
    when 0 then precision = precision <= 3 + hw ? hw : precision - 3
    when 1 then precision = precision <= 4 + hw ? hw : precision - 4
    when 2 then precision = precision <= 7 + hw ? hw : precision - 7
    when 3
      if aoff % 3600 == 0
        precision = precision <= 1 + hw ? hw : precision - 1
      elsif aoff % 60 == 0
        precision = precision <= 4 + hw ? hw : precision - 4
      else
        precision = precision <= 7 + hw ? hw : precision - 7
      end
    else
      return :unknown
    end
    if padding == " " && precision > hl
      out << " " * (precision - hl)
      precision = hl
    end
    if off < 0
      off = -off
      out << "-"
    else
      out << "+"
    end
    out << format("%.*d", precision, off / 3600)
    off %= 3600
    return if colons == 3 && off == 0
    out << ":" if colons >= 1
    out << format("%02d", off / 60)
    off %= 60
    return if colons == 3 && off == 0
    out << ":" << format("%02d", off) if colons >= 2
    nil
  end
  private :__fmt_z
end

class DateTime < Date
end

class Date
  class << DateTime
    def jd(*args)
      raise ArgumentError, "wrong number of arguments (given #{args.size}, expected 0..6)" if args.size > 6
      argc = args.size
      jd = 0
      h = min = s = 0
      fr2 = 0
      of = 0
      sg = ITALY
      sg = Calc.val2sg(args[5]) if argc >= 6
      of = Calc.val2off(args[4]) if argc >= 5
      s, fr2 = __int_frac(args[3], "second", 86400, argc, Float::INFINITY, fr2) if argc >= 4
      min, fr2 = __int_frac(args[2], "minute", 1440, argc, 3, fr2) if argc >= 3
      h, fr2 = __int_frac(args[1], "hour", 24, argc, 2, fr2) if argc >= 2
      if argc >= 1
        Calc.numeric!(args[0], "jd")
        jd, fr = Calc.trunc(args[0])
        if fr != 0
          raise Error, "invalid fraction" if argc > 1
          fr2 = fr
        end
      end
      __build(jd, h, min, s, of, sg, fr2)
    end

    def ordinal(*args)
      raise ArgumentError, "wrong number of arguments (given #{args.size}, expected 0..7)" if args.size > 7
      argc = args.size
      y = -4712
      d = 1
      h = min = s = 0
      fr2 = 0
      of = 0
      sg = ITALY
      sg = Calc.val2sg(args[6]) if argc >= 7
      of = Calc.val2off(args[5]) if argc >= 6
      s, fr2 = __int_frac(args[4], "second", 86400, argc, Float::INFINITY, fr2) if argc >= 5
      min, fr2 = __int_frac(args[3], "minute", 1440, argc, 4, fr2) if argc >= 4
      h, fr2 = __int_frac(args[2], "hour", 24, argc, 3, fr2) if argc >= 3
      d, fr2 = __int_frac(args[1], "yday", nil, argc, 2, fr2) if argc >= 2
      if argc >= 1
        Calc.numeric!(args[0], "year")
        y = Calc.num2int(args[0])
      end
      jd = Calc.valid_ordinal(y, d, sg) or raise Error, "invalid date"
      __build(jd, h, min, s, of, sg, fr2)
    end

    def civil(*args)
      raise ArgumentError, "wrong number of arguments (given #{args.size}, expected 0..8)" if args.size > 8
      argc = args.size
      y = -4712
      m = 1
      d = 1
      h = min = s = 0
      fr2 = 0
      of = 0
      sg = ITALY
      sg = Calc.val2sg(args[7]) if argc >= 8
      of = Calc.val2off(args[6]) if argc >= 7
      s, fr2 = __int_frac(args[5], "second", 86400, argc, Float::INFINITY, fr2) if argc >= 6
      min, fr2 = __int_frac(args[4], "minute", 1440, argc, 5, fr2) if argc >= 5
      h, fr2 = __int_frac(args[3], "hour", 24, argc, 4, fr2) if argc >= 4
      d, fr2 = __int_frac(args[2], "day", nil, argc, 3, fr2) if argc >= 3
      if argc >= 2
        Calc.numeric!(args[1], "month")
        m = Calc.num2int(args[1])
      end
      if argc >= 1
        Calc.numeric!(args[0], "year")
        y = args[0]
        y = y.to_i unless y.is_a?(Integer)
      end
      jd = Calc.valid_civil(y, m, d, sg) or raise Error, "invalid date"
      __build(jd, h, min, s, of, sg, fr2)
    end
    alias new civil

    def commercial(*args)
      raise ArgumentError, "wrong number of arguments (given #{args.size}, expected 0..8)" if args.size > 8
      argc = args.size
      y = -4712
      w = 1
      d = 1
      h = min = s = 0
      fr2 = 0
      of = 0
      sg = ITALY
      sg = Calc.val2sg(args[7]) if argc >= 8
      of = Calc.val2off(args[6]) if argc >= 7
      s, fr2 = __int_frac(args[5], "second", 86400, argc, Float::INFINITY, fr2) if argc >= 6
      min, fr2 = __int_frac(args[4], "minute", 1440, argc, 5, fr2) if argc >= 5
      h, fr2 = __int_frac(args[3], "hour", 24, argc, 4, fr2) if argc >= 4
      d, fr2 = __int_frac(args[2], "cwday", nil, argc, 3, fr2) if argc >= 3
      if argc >= 2
        Calc.numeric!(args[1], "cweek")
        w = Calc.num2int(args[1])
      end
      if argc >= 1
        Calc.numeric!(args[0], "year")
        y = Calc.num2int(args[0])
      end
      jd = Calc.valid_commercial(y, w, d, sg) or raise Error, "invalid date"
      __build(jd, h, min, s, of, sg, fr2)
    end

    def now(sg = ITALY)
      sg = Calc.num2dbl(sg)
      t = Time.at(*Process.clock_gettime(Process::CLOCK_REALTIME, :nanosecond).divmod(Calc::NS), :nsec)
      s = t.sec
      s = 59 if s == 60
      of = t.utc_offset
      of = 0 if of < -Calc::DAY || of > Calc::DAY
      ljd = Calc.civil_to_jd(t.year, t.mon, t.mday, -Float::INFINITY)
      jd, df = __local_to_utc(ljd, t.hour * 3600 + t.min * 60 + s, of)
      __new_raw(jd, df, t.nsec, of, sg, true)
    end

    private

    def __int_frac(v, field, per, argc, n, fr2)
      Calc.numeric!(v, field)
      i, fr = Calc.trunc(v, per)
      if fr != 0
        raise Error, "invalid fraction" if argc > n
        fr2 = fr
      end
      [Calc.num2int(i), fr2]
    end

    def __build(ljd, h, min, s, of, sg, fr2)
      t = Calc.valid_time(h, min, s) or raise Error, "invalid date"
      h, min, s = t
      if h == 24
        h = 0
        fr2 += 1
      end
      jd, df = __local_to_utc(ljd, h * 3600 + min * 60 + s, of)
      ret = __new_raw(jd, df, 0, of, sg, true)
      fr2 == 0 ? ret : ret + fr2
    end
  end
end

class DateTime
  def hour = __time[0]
  def min = __time[1]
  alias minute min
  def sec = __time[2]
  alias second sec
  public :hour, :min, :minute, :sec, :second
  def sec_fraction = __sf_in_sec
  alias second_fraction sec_fraction
  def offset = Rational(@cx ? @of : 0, Calc::DAY)
  def zone = __zone

  def new_offset(*args)
    raise ArgumentError, "wrong number of arguments (given #{args.size}, expected 0..1)" if args.size > 1
    of = args.empty? ? 0 : Calc.val2off(args[0])
    __with(of: of, cx: true)
  end

  def to_s = Calc.us(__strftime("%Y-%m-%dT%H:%M:%S%:z"))

  def strftime(*args)
    raise ArgumentError, "wrong number of arguments (given #{args.size}, expected 0..1)" if args.size > 1
    __strftime_api(args, "%Y-%m-%dT%H:%M:%S%:z")
  end

  def __timediv(n) # :nodoc:
    fmt = +"T%H:%M:%S"
    fmt << ".%#{n}N" if n > 0
    fmt << "%:z"
    __strftime(fmt)
  end
  private :__timediv

  def iso8601(n = 0)
    Calc.us(__strftime("%Y-%m-%d") << __timediv(Calc.num2int(n)))
  end
  alias xmlschema iso8601
  alias rfc3339 iso8601

  def jisx0301(n = 0)
    Calc.us(__strftime(__jisx0301_fmt) << __timediv(Calc.num2int(n)))
  end

  def deconstruct_keys(keys) = __deconstruct(keys, true)

  def to_time
    d = __julian? ? gregorian : self
    y, m, dd = d.__send__(:__civil)
    h, mi, sec = d.__send__(:__time)
    Time.new(y, m, dd, h, mi, sec + d.__send__(:__sf_in_sec), @cx ? @of : 0)
  end

  def to_date
    Date.__send__(:__new_raw, __ljd, 0, 0, 0, @sg, false)
  end

  def to_datetime = self
end

class Time
  unless method_defined?(:to_time)
    def to_time = self
  end

  def to_date
    Date.__send__(:__new_raw, Date.__send__(:__calc_civil_jd, year, mon, mday), 0, 0, 0, Date::ITALY, false)
  end

  def to_datetime
    s = sec
    s = 59 if s == 60
    of = utc_offset
    ljd = Date.__send__(:__calc_civil_jd, year, mon, mday)
    jd, df = DateTime.__send__(:__local_to_utc, ljd, hour * 3600 + min * 60 + s, of)
    sf = subsec * 1_000_000_000
    DateTime.__send__(:__new_raw, jd, df, sf, of, Date::ITALY, true)
  end
end

class Date
  def self.__calc_civil_jd(y, m, d) = Calc.civil_to_jd(y, m, d, -Float::INFINITY) # :nodoc:
  private_class_method :__calc_civil_jd
end

class Date
  # ---- parsing (date_parse.c, date_strptime.c) ----

  ZONETAB_SRC = # :nodoc:
    "ut=0;gmt=0;est=-18000;edt=-14400;cst=-21600;cdt=-18000;mst=-25200;mdt=-21600;pst=-28800;pdt=-25200;a=3600;" \
    "b=7200;c=10800;d=14400;e=18000;f=21600;g=25200;h=28800;i=32400;k=36000;l=39600;m=43200;n=-3600;o=-7200;" \
    "p=-10800;q=-14400;r=-18000;s=-21600;t=-25200;u=-28800;v=-32400;w=-36000;x=-39600;y=-43200;z=0;utc=0;wet=0;" \
    "at=-7200;brst=-7200;ndt=-5400;art=-10800;adt=-10800;brt=-10800;clst=-10800;nst=-9000;ast=-14400;clt=-14400;" \
    "akdt=-28800;ydt=-28800;akst=-32400;hadt=-32400;hdt=-32400;yst=-32400;ahst=-36000;cat=7200;hast=-36000;" \
    "hst=-36000;nt=-39600;idlw=-43200;bst=3600;cet=3600;fwt=3600;met=3600;mewt=3600;mez=3600;swt=3600;wat=3600;" \
    "west=3600;cest=7200;eet=7200;fst=7200;mest=7200;mesz=7200;sast=7200;sst=-39600;bt=10800;eat=10800;eest=10800;" \
    "msk=10800;msd=14400;zp4=14400;zp5=18000;ist=19800;zp6=21600;wast=7200;cct=23400;sgt=28800;wadt=28800;jst=32400;" \
    "kst=32400;east=-21600;gst=36000;eadt=39600;idle=43200;nzst=43200;nzt=43200;nzdt=46800;afghanistan=16200;" \
    "alaskan=-32400;arab=10800;arabian=14400;arabic=10800;atlantic=-14400;aus central=34200;aus eastern=36000;" \
    "azores=-3600;canada central=-21600;cape verde=-3600;caucasus=14400;cen. australia=34200;central america=-21600;" \
    "central asia=21600;central europe=3600;central european=3600;central pacific=39600;central=-21600;china=28800;" \
    "dateline=-43200;e. africa=10800;e. australia=36000;e. europe=7200;e. south america=-10800;eastern=-18000;" \
    "egypt=7200;ekaterinburg=18000;fiji=43200;fle=7200;greenland=-10800;greenwich=0;gtb=7200;hawaiian=-36000;" \
    "india=19800;iran=12600;jerusalem=7200;korea=32400;mexico=-21600;mid-atlantic=-7200;mountain=-25200;" \
    "myanmar=23400;n. central asia=21600;nepal=20700;new zealand=43200;newfoundland=-12600;north asia east=28800;" \
    "north asia=25200;pacific sa=-14400;pacific=-28800;romance=3600;russian=10800;sa eastern=-10800;" \
    "sa pacific=-18000;sa western=-14400;samoa=-39600;se asia=25200;malay peninsula=28800;south africa=7200;" \
    "sri lanka=21600;taipei=28800;tasmania=36000;tokyo=32400;tonga=46800;us eastern=-18000;us mountain=-25200;" \
    "vladivostok=36000;w. australia=28800;w. central africa=3600;w. europe=3600;west asia=18000;west pacific=36000;" \
    "yakutsk=32400;acdt=37800;acst=34200;act=-18000;acwst=31500;aedt=39600;aest=36000;aft=16200;almt=21600;" \
    "anast=43200;anat=43200;aoe=-43200;aqtt=18000;awdt=32400;awst=28800;azost=0;azot=-3600;azst=18000;azt=14400;" \
    "bnt=28800;bot=-14400;btt=21600;cast=28800;chadt=49500;chast=45900;chost=32400;chot=28800;chst=36000;chut=36000;" \
    "cidst=-14400;cist=-18000;ckt=-36000;cot=-18000;cvt=-3600;cxt=25200;davt=25200;ddut=36000;easst=-18000;" \
    "ect=-18000;egst=0;egt=-3600;fet=10800;fjst=46800;fjt=43200;fkst=-10800;fkt=-14400;fnt=-7200;galt=-21600;" \
    "gamt=-32400;get=14400;gft=-10800;gilt=43200;gyt=-14400;hkt=28800;hovst=28800;hovt=25200;ict=25200;idt=10800;" \
    "iot=21600;irdt=16200;irkst=32400;irkt=28800;irst=12600;kgt=21600;kost=39600;krast=28800;krat=25200;kuyt=14400;" \
    "lhdt=39600;lhst=37800;lint=50400;magst=43200;magt=39600;mart=-30600;mawt=18000;mht=43200;mmt=23400;mut=14400;" \
    "mvt=18000;myt=28800;nct=39600;nfdt=43200;nft=39600;novst=25200;novt=25200;npt=20700;nrt=43200;nut=-39600;" \
    "omsst=25200;omst=21600;orat=18000;pet=-18000;petst=43200;pett=43200;pgt=36000;phot=46800;pht=28800;pkt=18000;" \
    "pmdt=-7200;pmst=-10800;pont=39600;pwt=32400;pyst=-10800;qyzt=21600;ret=14400;rott=-10800;sakt=39600;samt=14400;" \
    "sbt=39600;sct=14400;sret=39600;srt=-10800;syot=10800;taht=-36000;tft=18000;tjt=18000;tkt=46800;tlt=32400;" \
    "tmt=18000;tost=50400;tot=46800;trt=10800;tvt=43200;ulast=32400;ulat=28800;uyst=-7200;uyt=-10800;uzt=18000;" \
    "vet=-14400;vlast=39600;vlat=36000;vost=21600;vut=39600;wakt=43200;warst=-10800;wft=43200;wgst=-3600;wgt=-7200;" \
    "wib=25200;wit=32400;wita=28800;wt=0;yakst=36000;yakt=32400;yapt=36000;yekst=21600;yekt=18000"
  ZONES = ZONETAB_SRC.split(";").to_h { |e| n, o = e.split("="); [n, o.to_i] }.freeze # :nodoc:
  ZONE_MAX = 17 # :nodoc:
  ABBR_DAYS_L = %w[sun mon tue wed thu fri sat].freeze # :nodoc:
  ABBR_MONTHS_L = %w[jan feb mar apr may jun jul aug sep oct nov dec].freeze # :nodoc:
  private_constant :ZONETAB_SRC, :ZONES, :ZONE_MAX, :ABBR_DAYS_L, :ABBR_MONTHS_L

  module Parse # :nodoc:
    DAYS = "sunday|monday|tuesday|wednesday|thursday|friday|saturday"
    ABBR_DAYS = "sun|mon|tue|wed|thu|fri|sat"
    ABBR_MONTHS = "jan|feb|mar|apr|may|jun|jul|aug|sep|oct|nov|dec"
    NUMBER = "(?<!\\d)\\d"
    BEGIN_ERA = "\\b"
    END_ERA = "(?!(?<!\\.)[a-z])"
    ERA = "(c(?:e|\\.e\\.)|b(?:ce|\\.c\\.e\\.)|a(?:d|\\.d\\.)|b(?:c|\\.c\\.))"

    PAT_DAY = /\b(#{ABBR_DAYS})[^-\/\d\s]*/i
    PAT_TIME = Regexp.new(
      "(" \
        "#{NUMBER}+\\s*" \
        "(?:" \
          "(?:" \
            ":\\s*\\d+" \
            "(?:" \
              "\\s*:\\s*\\d+(?:[,.]\\d*)?" \
            ")?" \
          "|" \
            "h(?:\\s*\\d+m?(?:\\s*\\d+s?)?)?" \
          ")" \
          "(?:" \
            "\\s*" \
            "[ap](?:m\\b|\\.m\\.)" \
          ")?" \
        "|" \
          "[ap](?:m\\b|\\.m\\.)" \
        ")" \
      ")" \
      "(?:" \
        "\\s*" \
        "(" \
          "(?:gmt|utc?)?[-+]\\d+(?:[,.:]\\d+(?::\\d+)?)?" \
        "|" \
          "(?-i:[[:alpha:].\\s]+)(?:standard|daylight)\\stime\\b" \
        "|" \
          "(?-i:[[:alpha:]]+)(?:\\sdst)?\\b" \
        ")" \
      ")?", Regexp::IGNORECASE)
    PAT_TIME2 = /\A(\d+)h?(?:\s*:?\s*(\d+)m?(?:\s*:?\s*(\d+)(?:[,.](\d+))?s?)?)?(?:\s*([ap])(?:m\b|\.m\.))?/i
    PAT_EU = Regexp.new(
      "('?#{NUMBER}+)[^-\\d\\s]*" \
      "\\s*" \
      "(#{ABBR_MONTHS})[^-\\d\\s']*" \
      "(?:" \
        "\\s*" \
        "(?:" \
          "#{BEGIN_ERA}#{ERA}#{END_ERA}" \
        ")?" \
        "\\s*" \
        "('?-?\\d+(?:(?:st|nd|rd|th)\\b)?)" \
      ")?", Regexp::IGNORECASE)
    PAT_US = Regexp.new(
      "\\b(#{ABBR_MONTHS})[^-\\d\\s']*" \
      "\\s*" \
      "('?\\d+)[^-\\d\\s']*" \
      "(?:" \
        "\\s*+,?" \
        "\\s*+" \
        "#{ERA}?" \
        "\\s*" \
        "('?-?\\d+)" \
      ")?", Regexp::IGNORECASE)
    PAT_ISO = /('?[-+]?#{NUMBER}+)-(\d+)-('?-?\d+)/
    PAT_ISO21 = /\b(\d{2}|\d{4})?-?w(\d{2})(?:-?(\d))?\b/i
    PAT_ISO22 = /-w-(\d)\b/i
    PAT_ISO23 = /--(\d{2})?-(\d{2})\b/
    PAT_ISO24 = /--(\d{2})(\d{2})?\b/
    PAT_ISO25_0 = /[,.](\d{2}|\d{4})-\d{3}\b/
    PAT_ISO25 = /\b(\d{2}|\d{4})-(\d{3})\b/
    PAT_ISO26_0 = /\d-\d{3}\b/
    PAT_ISO26 = /\b-(\d{3})\b/
    PAT_JIS = /\b([mtshr])(\d+)\.(\d+)\.(\d+)/i
    PAT_VMS11 = /('?-?#{NUMBER}+)-(#{ABBR_MONTHS})[^-\/.]*-('?-?\d+)/i
    PAT_VMS12 = /\b(#{ABBR_MONTHS})[^-\/.]*-('?-?\d+)(?:-('?-?\d+))?/i
    PAT_SLA = /('?-?#{NUMBER}+)\/\s*('?\d+)(?:\D\s*('?-?\d+))?/i
    PAT_DOT = /('?-?#{NUMBER}+)\.\s*('?\d+)\.\s*('?-?\d+)/i
    PAT_YEAR = /'(\d+)\b/
    PAT_MON = /\b(#{ABBR_MONTHS})\S*/i
    PAT_MDAY = /(#{NUMBER}+)(st|nd|rd|th)\b/i
    PAT_DDD = Regexp.new(
      "([-+]?)(#{NUMBER}{2,14})" \
      "(?:" \
        "\\s*" \
        "t?" \
        "\\s*" \
        "(\\d{2,6})?(?:[,.](\\d*))?" \
      ")?" \
      "(?:" \
        "\\s*" \
        "(" \
          "z\\b" \
        "|" \
          "[-+]\\d{1,4}\\b" \
        "|" \
          "\\[[-+]?\\d[^\\]]*\\]" \
        ")" \
      ")?", Regexp::IGNORECASE)
    PAT_BC = /\b(bc\b|bce\b|b\.c\.|b\.c\.e\.)/i
    PAT_FRAG = /\A\s*(\d{1,2})\s*\z/i
    PAT_CLEAN = /[^-+',.\/:@[:alnum:]\[\]]+/

    ISO8601_EXT_DATETIME = /\A\s*(?:([-+]?\d{2,}|-)-(\d{2})?(?:-(\d{2}))?|([-+]?\d{2,})?-(\d{3})|(\d{4}|\d{2})?-w(\d{2})-(\d)|-w-(\d))(?:t(\d{2}):(\d{2})(?::(\d{2})(?:[,.](\d+))?)?(z|[-+]\d{2}(?::?\d{2})?)?)?\s*\z/i
    ISO8601_BAS_DATETIME = /\A\s*(?:([-+]?(?:\d{4}|\d{2})|--)(\d{2}|-)(\d{2})|([-+]?(?:\d{4}|\d{2}))(\d{3})|-(\d{3})|(\d{4}|\d{2})w(\d{2})(\d)|-w(\d{2})(\d)|-w-(\d))(?:t?(\d{2})(\d{2})(?:(\d{2})(?:[,.](\d+))?)?(z|[-+]\d{2}(?:\d{2})?)?)?\s*\z/i
    ISO8601_EXT_TIME = /\A\s*(\d{2}):(\d{2})(?::(\d{2})(?:[,.](\d+))?(z|[-+]\d{2}(:?\d{2})?)?)?\s*\z/i
    ISO8601_BAS_TIME = /\A\s*(\d{2})(\d{2})(?:(\d{2})(?:[,.](\d+))?(z|[-+]\d{2}(\d{2})?)?)?\s*\z/i
    RFC3339 = /\A\s*(-?\d{4})-(\d{2})-(\d{2})(t|\s)(\d{2}):(\d{2}):(\d{2})(?:\.(\d+))?(z|[-+]\d{2}:\d{2})\s*\z/i
    XMLSCHEMA_DATETIME = /\A\s*(-?\d{4,})(?:-(\d{2})(?:-(\d{2}))?)?(?:t(\d{2}):(\d{2}):(\d{2})(?:\.(\d+))?)?(z|[-+]\d{2}:\d{2})?\s*\z/i
    XMLSCHEMA_TIME = /\A\s*(\d{2}):(\d{2}):(\d{2})(?:\.(\d+))?(z|[-+]\d{2}:\d{2})?\s*\z/i
    XMLSCHEMA_TRUNC = /\A\s*(?:--(\d{2})(?:-(\d{2}))?|---(\d{2}))(z|[-+]\d{2}:\d{2})?\s*\z/i
    RFC2822 = /\A\s*(?:(#{ABBR_DAYS})\s*,\s+)?(\d{1,2})\s+(#{ABBR_MONTHS})\s+(-?\d{2,})\s+(\d{2}):(\d{2})(?::(\d{2}))?\s*([-+]\d{4}|ut|gmt|e[sd]t|c[sd]t|m[sd]t|p[sd]t|[a-ik-z])\s*\z/i
    HTTPDATE1 = /\A\s*(#{ABBR_DAYS})\s*,\s+(\d{2})\s+(#{ABBR_MONTHS})\s+(-?\d{4})\s+(\d{2}):(\d{2}):(\d{2})\s+(gmt)\s*\z/i
    HTTPDATE2 = /\A\s*(#{DAYS})\s*,\s+(\d{2})\s*-\s*(#{ABBR_MONTHS})\s*-\s*(\d{2})\s+(\d{2}):(\d{2}):(\d{2})\s+(gmt)\s*\z/i
    HTTPDATE3 = /\A\s*(#{ABBR_DAYS})\s+(#{ABBR_MONTHS})\s+(\d{1,2})\s+(\d{2}):(\d{2}):(\d{2})\s+(\d{4})\s*\z/i
    JISX0301 = /\A\s*([mtshr])?(\d{2})\.(\d{2})\.(\d{2})(?:t(?:(\d{2}):(\d{2})(?::(\d{2})(?:[,.](\d*))?)?(z|[-+]\d{2}(?::?\d{2})?)?)?)?\s*\z/i
    STRPTIME_ZONE = /\A((?:gmt|utc?)?[-+]\d+(?:[,.:]\d+(?::\d+)?)?|(?-i:[[:alpha:].\s]+)(?:standard|daylight)\s+time\b|(?-i:[[:alpha:]]+)(?:\s+dst)?\b)/i

    module_function

    def space?(c) = c == 32 || (c && c >= 9 && c <= 13)
    def digit?(c) = c && c >= 48 && c <= 57
    def sign?(c) = c == 43 || c == 45

    def str_end_with_word(s, l, w)
      n = w.bytesize
      return 0 if l <= n || !space?(s.getbyte(l - n - 1))
      return 0 unless s.byteslice(l - n, n).casecmp?(w)
      n += 1
      n += 1 while l > n && space?(s.getbyte(l - n - 1))
      n
    end

    def shrunk_size(s, l)
      ni = 0
      sp = false
      l.times do |i|
        if space?(s.getbyte(i))
          sp = true
        else
          ni += 1 if sp
          sp = false
          ni += 1
        end
      end
      ni < l ? ni : 0
    end

    def shrink_space(s, l)
      d = (+"").b
      sp = false
      l.times do |i|
        c = s.getbyte(i)
        if space?(c)
          sp = true
        else
          d << " " if sp
          sp = false
          d << c.chr
        end
      end
      d
    end

    # strtoul: [value, index after the digits] (no digits: [0, start])
    def strtoul(s, i)
      j = i
      j += 1 while space?(s.getbyte(j))
      j += 1 if sign?(s.getbyte(j))
      k = j
      k += 1 while digit?(s.getbyte(k))
      return [0, i] if k == j
      [s.byteslice(j, k - j).to_i, k]
    end

    # ruby_scan_digits: [value, number of digits read], at most len
    def scan_digits(s, i, len)
      k = 0
      k += 1 while k < len && digit?(s.getbyte(i + k))
      [k == 0 ? 0 : s.byteslice(i, k).to_i, k]
    end

    def zone_to_diff(str)
      s = str.b
      l = s.bytesize
      dst = false
      if (w = str_end_with_word(s, l, "time")) > 0
        wtime = w
        l -= w
        if (w = str_end_with_word(s, l, "standard")) > 0
          l -= w
        elsif (w = str_end_with_word(s, l, "daylight")) > 0
          l -= w
          dst = true
        else
          l += wtime
        end
      elsif (w = str_end_with_word(s, l, "dst")) > 0
        l -= w
        dst = true
      end
      zn = s
      sl = shrunk_size(s, l)
      if sl <= 0
        sl = l
      elsif sl <= ZONE_MAX
        zn = shrink_space(s, l)
        sl = zn.bytesize
      end
      if sl > 0 && sl <= ZONE_MAX
        z = ZONES[zn.byteslice(0, sl).downcase]
        return dst ? z + 3600 : z if z
      end
      i = 0
      if l > 3 && (s.byteslice(0, 3).casecmp?("gmt") || s.byteslice(0, 3).casecmp?("utc"))
        i += 3
        l -= 3
      end
      return nil unless sign?(s.getbyte(i))
      neg = s.getbyte(i) == 45
      i += 1
      l -= 1
      e = i + l
      hour = min = sec = 0
      hour, p = strtoul(s, i)
      c = s.getbyte(p)
      if c == 58 # ':'
        return nil if hour > 23
        min, p = strtoul(s, p + 1)
        return nil if min > 59
        if s.getbyte(p) == 58
          sec, p = strtoul(s, p + 1)
          return nil if sec > 59
        end
      elsif c == 44 || c == 46 # ',' '.'
        return nil if hour > 23
        p += 1
        n = e - p
        n = 7 if n > 7
        sec, n = scan_digits(s, p, n)
        p += n
        nc = s.getbyte(p)
        sec += 1 if p < e && nc && nc >= 53 + (sec.odd? ? 0 : 1) && nc <= 57
        sec *= 36
        if neg
          hour = -hour
          sec = -sec
        end
        if n <= 2
          sec *= 10 if n == 1
          return sec + hour * 3600
        end
        off = Rational(sec, 10**(n - 2)) + hour * 3600
        return off.denominator == 1 ? off.numerator : off
      elsif l > 2
        hour, = scan_digits(s, i, 2 - l % 2)
        min, = scan_digits(s, i + 2 - l % 2, 2) if l >= 3
        sec, = scan_digits(s, i + 4 - l % 2, 2) if l >= 5
      end
      sec += min * 60 + hour * 3600
      neg ? -sec : sec
    end

    def day_num(s)
      t = s[0, 3]
      ABBR_DAYS_L.index { |d| d.casecmp?(t) } || 7
    end

    def mon_num(s)
      t = s[0, 3]
      (ABBR_MONTHS_L.index { |m| m.casecmp?(t) } || 12) + 1
    end

    def subs(str, pat)
      m = pat.match(str) or return false
      b = m.begin(0)
      str[b, m.end(0) - b] = " "
      yield m
      true
    end

    def first_index(s, sign_ok)
      s.each_char.with_index { |c, i| return i if c.match?(/\d/) || (sign_ok && (c == "-" || c == "+")) }
      nil
    end

    def digits_from(s, i)
      j = i
      j += 1 while j < s.size && s[j].match?(/\d/)
      j
    end

    def s3e(h, y, m, d, bc)
      c = nil
      m = m.to_s unless m.is_a?(String)
      if y && m && d.nil?
        y, m, d = d, y, m
      end
      if y.nil?
        if d && d.size > 2
          y = d
          d = nil
        end
        if d && d.size > 0 && d[0] == "'"
          y = d
          d = nil
        end
      end
      if y && (bp = first_index(y, true))
        s = bp
        s += 1 if y[s] == "-" || y[s] == "+"
        ep = digits_from(y, s)
        if ep < y.size
          y, d = d, y[bp...ep]
        end
      end
      if m && (m[0] == "'" || m.size > 2)
        y, m, d = m, d, y
      end
      if d && (d[0] == "'" || d.size > 2)
        y, d = d, y
      end
      if y && (bp = first_index(y, true))
        s = bp
        if y[s] == "-" || y[s] == "+"
          s += 1
          c = false
        end
        ep = digits_from(y, s)
        c = false if ep - s > 2
        h[:year] = y[bp...ep].to_i
      end
      h[:_bc] = true if bc
      if m && (bp = first_index(m, false))
        h[:mon] = m[bp...digits_from(m, bp)].to_i
      end
      if d && (bp = first_index(d, false))
        h[:mday] = d[bp...digits_from(d, bp)].to_i
      end
      h[:_comp] = c unless c.nil?
    end

    def bc?(b) = !b.nil? && (b[0] == "B" || b[0] == "b")

    def parse_time2(m, h)
      hour = m[1].to_i
      min = m[2] && m[2].to_i
      s = m[3] && m[3].to_i
      f = m[4] && Rational(m[4].to_i, 10**m[4].size)
      if (p = m[5])
        hour %= 12
        hour += 12 if p[0] == "P" || p[0] == "p"
      end
      h[:hour] = hour
      h[:min] = min if min
      h[:sec] = s if s
      h[:sec_fraction] = f if f
    end

    def parse_ddd(m, h)
      s1, s2, s3, s4, s5 = m[1], m[2], m[3], m[4], m[5]
      n2i = ->(s, f, w) { s[f, w].to_i }
      neg = s1 && s1[0] == "-"
      l2 = s2.size
      tm = s3.nil? && !s4.nil?
      case l2
      when 2
        if tm then h[:sec] = n2i.(s2, l2 - 2, 2) else h[:mday] = n2i.(s2, 0, 2) end
      when 4
        if tm
          h[:sec] = n2i.(s2, l2 - 2, 2)
          h[:min] = n2i.(s2, l2 - 4, 2)
        else
          h[:mon] = n2i.(s2, 0, 2)
          h[:mday] = n2i.(s2, 2, 2)
        end
      when 6
        if tm
          h[:sec] = n2i.(s2, l2 - 2, 2)
          h[:min] = n2i.(s2, l2 - 4, 2)
          h[:hour] = n2i.(s2, l2 - 6, 2)
        else
          y = n2i.(s2, 0, 2)
          y = -y if neg
          h[:year] = y
          h[:mon] = n2i.(s2, 2, 2)
          h[:mday] = n2i.(s2, 4, 2)
        end
      when 8, 10, 12, 14
        if tm
          h[:sec] = n2i.(s2, l2 - 2, 2)
          h[:min] = n2i.(s2, l2 - 4, 2)
          h[:hour] = n2i.(s2, l2 - 6, 2)
          h[:mday] = n2i.(s2, l2 - 8, 2)
          h[:mon] = n2i.(s2, l2 - 10, 2) if l2 >= 10
          if l2 == 12
            y = n2i.(s2, l2 - 12, 2)
            y = -y if neg
            h[:year] = y
          end
          if l2 == 14
            y = n2i.(s2, l2 - 14, 4)
            y = -y if neg
            h[:year] = y
            h[:_comp] = false
          end
        else
          y = n2i.(s2, 0, 4)
          y = -y if neg
          h[:year] = y
          h[:mon] = n2i.(s2, 4, 2)
          h[:mday] = n2i.(s2, 6, 2)
          h[:hour] = n2i.(s2, 8, 2) if l2 >= 10
          h[:min] = n2i.(s2, 10, 2) if l2 >= 12
          h[:sec] = n2i.(s2, 12, 2) if l2 >= 14
          h[:_comp] = false
        end
      when 3
        if tm
          h[:sec] = n2i.(s2, l2 - 2, 2)
          h[:min] = n2i.(s2, l2 - 3, 1)
        else
          h[:yday] = n2i.(s2, 0, 3)
        end
      when 5
        if tm
          h[:sec] = n2i.(s2, l2 - 2, 2)
          h[:min] = n2i.(s2, l2 - 4, 2)
          h[:hour] = n2i.(s2, l2 - 5, 1)
        else
          y = n2i.(s2, 0, 2)
          y = -y if neg
          h[:year] = y
          h[:yday] = n2i.(s2, 2, 3)
        end
      when 7
        if tm
          h[:sec] = n2i.(s2, l2 - 2, 2)
          h[:min] = n2i.(s2, l2 - 4, 2)
          h[:hour] = n2i.(s2, l2 - 6, 2)
          h[:mday] = n2i.(s2, l2 - 7, 1)
        else
          y = n2i.(s2, 0, 4)
          y = -y if neg
          h[:year] = y
          h[:yday] = n2i.(s2, 4, 3)
        end
      end
      if s3
        l3 = s3.size
        if [2, 4, 6].include?(l3)
          if s4
            h[:sec] = n2i.(s3, l3 - 2, 2)
            h[:min] = n2i.(s3, l3 - 4, 2) if l3 >= 4
            h[:hour] = n2i.(s3, l3 - 6, 2) if l3 >= 6
          else
            h[:hour] = n2i.(s3, 0, 2)
            h[:min] = n2i.(s3, 2, 2) if l3 >= 4
            h[:sec] = n2i.(s3, 4, 2) if l3 >= 6
          end
        end
      end
      h[:sec_fraction] = Rational(s4.to_i, 10**s4.size) if s4
      if s5
        h[:zone] = s5
        if s5[0] == "["
          l5 = s5.size - 2
          inner = s5[1, l5]
          if (ci = inner.index(":"))
            zone = inner[ci + 1..]
            s5 = s5[1, ci + 1]
          else
            zone = inner
            s5 = inner[0].match?(/\d/) ? "+" + zone : zone
          end
          h[:zone] = zone
          h[:offset] = zone_to_diff(s5)
        end
      end
    end

    HAVE_ALPHA = 1
    HAVE_DIGIT = 2
    HAVE_DASH = 4
    HAVE_DOT = 8
    HAVE_SLASH = 16

    def check_class(s)
      f = 0
      s.each_byte do |c|
        f |= HAVE_ALPHA if (c >= 65 && c <= 90) || (c >= 97 && c <= 122)
        f |= HAVE_DIGIT if c >= 48 && c <= 57
        f |= HAVE_DASH if c == 45
        f |= HAVE_DOT if c == 46
        f |= HAVE_SLASH if c == 47
      end
      f
    end

    def parse(str, comp)
      str = str.gsub(PAT_CLEAN, " ")
      h = {}
      h[:_comp] = comp
      has = ->(x) { (check_class(str) & x) == x }
      subs(str, PAT_DAY) { |m| h[:wday] = day_num(m[1]) } if has.(HAVE_ALPHA)
      if has.(HAVE_DIGIT)
        subs(str, PAT_TIME) do |m|
          h[:zone] = m[2] if m[2]
          m2 = PAT_TIME2.match(m[1])
          parse_time2(m2, h) if m2
        end
      end
      catch(:ok) do
        if has.(HAVE_ALPHA | HAVE_DIGIT)
          throw :ok if subs(str, PAT_EU) { |m| s3e(h, m[4], mon_num(m[2]), m[1], bc?(m[3])) }
          throw :ok if subs(str, PAT_US) { |m| s3e(h, m[4], mon_num(m[1]), m[2], bc?(m[3])) }
        end
        if has.(HAVE_DIGIT | HAVE_DASH)
          throw :ok if subs(str, PAT_ISO) { |m| s3e(h, m[1], m[2], m[3], false) }
        end
        if has.(HAVE_DIGIT | HAVE_DOT)
          throw :ok if subs(str, PAT_JIS) { |m|
            ep = { "m" => 1867, "t" => 1911, "s" => 1925, "h" => 1988, "r" => 2018 }[m[1].downcase] || 0
            h[:year] = m[2].to_i + ep
            h[:mon] = m[3].to_i
            h[:mday] = m[4].to_i
          }
        end
        if has.(HAVE_ALPHA | HAVE_DIGIT | HAVE_DASH)
          throw :ok if subs(str, PAT_VMS11) { |m| s3e(h, m[3], mon_num(m[2]), m[1], false) }
          throw :ok if subs(str, PAT_VMS12) { |m| s3e(h, m[3], mon_num(m[1]), m[2], false) }
        end
        if has.(HAVE_DIGIT | HAVE_SLASH)
          throw :ok if subs(str, PAT_SLA) { |m| s3e(h, m[1], m[2], m[3], false) }
        end
        if has.(HAVE_DIGIT | HAVE_DOT)
          throw :ok if subs(str, PAT_DOT) { |m| s3e(h, m[1], m[2], m[3], false) }
        end
        if has.(HAVE_DIGIT)
          throw :ok if parse_iso2(str, h)
        end
        if has.(HAVE_DIGIT)
          throw :ok if subs(str, PAT_YEAR) { |m| h[:year] = m[1].to_i }
        end
        if has.(HAVE_ALPHA)
          throw :ok if subs(str, PAT_MON) { |m| h[:mon] = mon_num(m[1]) }
        end
        if has.(HAVE_DIGIT)
          throw :ok if subs(str, PAT_MDAY) { |m| h[:mday] = m[1].to_i }
        end
        if has.(HAVE_DIGIT)
          throw :ok if subs(str, PAT_DDD) { |m| parse_ddd(m, h) }
        end
      end
      subs(str, PAT_BC) { h[:_bc] = true } if has.(HAVE_ALPHA)
      if has.(HAVE_DIGIT)
        subs(str, PAT_FRAG) do |m|
          s = m[1]
          if !h[:hour].nil? && h[:mday].nil?
            n = s.to_i
            h[:mday] = n if n >= 1 && n <= 31
          end
          if !h[:mday].nil? && h[:hour].nil?
            n = s.to_i
            h[:hour] = n if n >= 0 && n <= 24
          end
        end
      end
      if h.delete(:_bc)
        h[:cwyear] = -h[:cwyear] + 1 unless h[:cwyear].nil?
        h[:year] = -h[:year] + 1 unless h[:year].nil?
      end
      if h.delete(:_comp)
        [:cwyear, :year].each do |k|
          y = h[k]
          h[k] = y + (y >= 69 ? 1900 : 2000) if !y.nil? && y >= 0 && y <= 99
        end
      end
      h[:offset] = zone_to_diff(h[:zone]) if !h[:zone].nil? && h[:offset].nil?
      h
    end

    def parse_iso2(str, h)
      return true if subs(str, PAT_ISO21) { |m|
        h[:cwyear] = m[1].to_i if m[1]
        h[:cweek] = m[2].to_i
        h[:cwday] = m[3].to_i if m[3]
      }
      return true if subs(str, PAT_ISO22) { |m| h[:cwday] = m[1].to_i }
      return true if subs(str, PAT_ISO23) { |m|
        h[:mon] = m[1].to_i if m[1]
        h[:mday] = m[2].to_i
      }
      return true if subs(str, PAT_ISO24) { |m|
        h[:mon] = m[1].to_i
        h[:mday] = m[2].to_i if m[2]
      }
      unless PAT_ISO25_0.match?(str)
        return true if subs(str, PAT_ISO25) { |m|
          h[:year] = m[1].to_i
          h[:yday] = m[2].to_i
        }
      end
      unless PAT_ISO26_0.match?(str)
        return true if subs(str, PAT_ISO26) { |m| h[:yday] = m[1].to_i }
      end
      false
    end

    def comp_year69(y) = y + (y >= 69 ? 1900 : 2000)
    def comp_year50(y) = y + (y >= 50 ? 1900 : 2000)
    def sec_fraction(f) = Rational(f.to_i, 10**f.size)

    def set_zone(h, z)
      h[:zone] = z
      h[:offset] = zone_to_diff(z)
    end

    def iso8601(str)
      h = {}
      if (m = ISO8601_EXT_DATETIME.match(str))
        s = m.to_a
        if s[1]
          h[:mday] = s[3].to_i if s[3]
          if s[1] != "-"
            y = s[1].to_i
            y = comp_year69(y) if s[1].size < 4
            h[:year] = y
          end
          if s[2].nil?
            return h if s[1] != "-"
          else
            h[:mon] = s[2].to_i
          end
        elsif s[5]
          h[:yday] = s[5].to_i
          if s[4]
            y = s[4].to_i
            y = comp_year69(y) if s[4].size < 4
            h[:year] = y
          end
        elsif s[8]
          h[:cweek] = s[7].to_i
          h[:cwday] = s[8].to_i
          if s[6]
            y = s[6].to_i
            y = comp_year69(y) if s[6].size < 4
            h[:cwyear] = y
          end
        elsif s[9]
          h[:cwday] = s[9].to_i
        end
        if s[10]
          h[:hour] = s[10].to_i
          h[:min] = s[11].to_i
          h[:sec] = s[12].to_i if s[12]
        end
        h[:sec_fraction] = sec_fraction(s[13]) if s[13]
        set_zone(h, s[14]) if s[14]
      elsif (m = ISO8601_BAS_DATETIME.match(str))
        s = m.to_a
        if s[3]
          h[:mday] = s[3].to_i
          if s[1] != "--"
            y = s[1].to_i
            y = comp_year69(y) if s[1].size < 4
            h[:year] = y
          end
          if s[2][0] == "-"
            return h if s[1] != "--"
          else
            h[:mon] = s[2].to_i
          end
        elsif s[5]
          h[:yday] = s[5].to_i
          y = s[4].to_i
          y = comp_year69(y) if s[4].size < 4
          h[:year] = y
        elsif s[6]
          h[:yday] = s[6].to_i
        elsif s[9]
          h[:cweek] = s[8].to_i
          h[:cwday] = s[9].to_i
          y = s[7].to_i
          y = comp_year69(y) if s[7].size < 4
          h[:cwyear] = y
        elsif s[11]
          h[:cweek] = s[10].to_i
          h[:cwday] = s[11].to_i
        elsif s[12]
          h[:cwday] = s[12].to_i
        end
        if s[13]
          h[:hour] = s[13].to_i
          h[:min] = s[14].to_i
          h[:sec] = s[15].to_i if s[15]
        end
        h[:sec_fraction] = sec_fraction(s[16]) if s[16]
        set_zone(h, s[17]) if s[17]
      elsif (m = ISO8601_EXT_TIME.match(str) || ISO8601_BAS_TIME.match(str))
        h[:hour] = m[1].to_i
        h[:min] = m[2].to_i
        h[:sec] = m[3].to_i if m[3]
        h[:sec_fraction] = sec_fraction(m[4]) if m[4]
        set_zone(h, m[5]) if m[5]
      end
      h
    end

    def rfc3339(str)
      h = {}
      if (m = RFC3339.match(str))
        c = m[4][0]
        unless c == "t" || c == "T" || c == " "
          warn("Invalid RFC3339 date time separator provided, this will raise an error in a future version.", category: :deprecated, uplevel: 3)
        end
        h[:year] = m[1].to_i
        h[:mon] = m[2].to_i
        h[:mday] = m[3].to_i
        h[:hour] = m[5].to_i
        h[:min] = m[6].to_i
        h[:sec] = m[7].to_i
        set_zone(h, m[9])
        h[:sec_fraction] = sec_fraction(m[8]) if m[8]
      end
      h
    end

    def xmlschema(str)
      h = {}
      if (m = XMLSCHEMA_DATETIME.match(str))
        h[:year] = m[1].to_i
        h[:mon] = m[2].to_i if m[2]
        h[:mday] = m[3].to_i if m[3]
        h[:hour] = m[4].to_i if m[4]
        h[:min] = m[5].to_i if m[5]
        h[:sec] = m[6].to_i if m[6]
        h[:sec_fraction] = sec_fraction(m[7]) if m[7]
        set_zone(h, m[8]) if m[8]
      elsif (m = XMLSCHEMA_TIME.match(str))
        h[:hour] = m[1].to_i
        h[:min] = m[2].to_i
        h[:sec] = m[3].to_i if m[3]
        h[:sec_fraction] = sec_fraction(m[4]) if m[4]
        set_zone(h, m[5]) if m[5]
      elsif (m = XMLSCHEMA_TRUNC.match(str))
        h[:mon] = m[1].to_i if m[1]
        h[:mday] = m[2].to_i if m[2]
        h[:mday] = m[3].to_i if m[3]
        set_zone(h, m[4]) if m[4]
      end
      h
    end

    def rfc2822(str)
      h = {}
      if (m = RFC2822.match(str))
        h[:wday] = day_num(m[1]) if m[1]
        h[:mday] = m[2].to_i
        h[:mon] = mon_num(m[3])
        y = m[4].to_i
        y = comp_year50(y) if m[4].size < 4
        h[:year] = y
        h[:hour] = m[5].to_i
        h[:min] = m[6].to_i
        h[:sec] = m[7].to_i if m[7]
        set_zone(h, m[8])
      end
      h
    end

    def httpdate(str)
      h = {}
      if (m = HTTPDATE1.match(str))
        h[:wday] = day_num(m[1])
        h[:mday] = m[2].to_i
        h[:mon] = mon_num(m[3])
        h[:year] = m[4].to_i
        h[:hour] = m[5].to_i
        h[:min] = m[6].to_i
        h[:sec] = m[7].to_i
        h[:zone] = m[8]
        h[:offset] = 0
      elsif (m = HTTPDATE2.match(str))
        h[:wday] = day_num(m[1])
        h[:mday] = m[2].to_i
        h[:mon] = mon_num(m[3])
        y = m[4].to_i
        y = comp_year69(y) if y >= 0 && y <= 99
        h[:year] = y
        h[:hour] = m[5].to_i
        h[:min] = m[6].to_i
        h[:sec] = m[7].to_i
        h[:zone] = m[8]
        h[:offset] = 0
      elsif (m = HTTPDATE3.match(str))
        h[:wday] = day_num(m[1])
        h[:mon] = mon_num(m[2])
        h[:mday] = m[3].to_i
        h[:hour] = m[4].to_i
        h[:min] = m[5].to_i
        h[:sec] = m[6].to_i
        h[:year] = m[7].to_i
      end
      h
    end

    def jisx0301(str)
      m = JISX0301.match(str) or return iso8601(str)
      h = {}
      ep = { "m" => 1867, "t" => 1911, "s" => 1925, "h" => 1988, "r" => 2018 }[(m[1] || "h").downcase]
      h[:year] = m[2].to_i + ep
      h[:mon] = m[3].to_i
      h[:mday] = m[4].to_i
      if m[5]
        h[:hour] = m[5].to_i
        h[:min] = m[6].to_i if m[6]
        h[:sec] = m[7].to_i if m[7]
      end
      h[:sec_fraction] = sec_fraction(m[8]) if m[8]
      set_zone(h, m[9]) if m[9]
      h
    end

    # ---- strptime ----

    DAY_NAMES = %w[Sunday Monday Tuesday Wednesday Thursday Friday Saturday].freeze
    MONTH_NAMES = %w[January February March April May June July August September October November December].freeze

    def num_pattern?(fmt, i)
      c = fmt.getbyte(i)
      return false unless c
      return true if digit?(c)
      if c == 37
        i += 1
        c = fmt.getbyte(i)
        if c == 69 || c == 79 # E O
          i += 1
          c = fmt.getbyte(i)
        end
        return true if c && ("CDdeFGgHIjkLlMmNQRrSsTUuVvWwXxYy".include?(c.chr) || digit?(c))
      end
      false
    end

    def read_digits(str, si, width)
      return nil if width == 0
      l = 0
      l += 1 while si + l < str.bytesize && digit?(str.getbyte(si + l)) && l < width
      return nil if l == 0
      [str.byteslice(si, l).to_i, l]
    end

    def head_match?(str, si, name, len)
      str.bytesize - si >= len && str.byteslice(si, len).casecmp?(name[0, len])
    end

    # returns the new si, or nil after setting h[:_fail]
    def strptime_internal(str, si, fmt, h, enc)
      slen = str.bytesize
      flen = fmt.bytesize
      fi = 0
      fail = -> { h[:_fail] = true; return nil }
      while fi < flen
        if space?(fmt.getbyte(fi))
          si += 1 while si < slen && space?(str.getbyte(si))
          fi += 1 while fi + 1 < flen && space?(fmt.getbyte(fi + 1))
          fi += 1
          next
        end
        return fail.() if si >= slen
        if fmt.getbyte(fi) == 37
          ordinal = false
          loop do
            fi += 1
            c = fmt.getbyte(fi)
            ch = c ? c.chr : "\0"
            case ch
            when "E"
              nx = fmt.getbyte(fi + 1)
              next if nx && "cCxXyY".include?(nx.chr)
              fi -= 1
              ordinal = true
            when "O"
              nx = fmt.getbyte(fi + 1)
              next if nx && "deHImMSuUVwWy".include?(nx.chr)
              fi -= 1
              ordinal = true
            when ":"
              i = 1
              i += 1 while i < 3 && fi + i < flen && fmt.getbyte(fi + i) == 58
              if fmt.getbyte(fi + i) == 122
                fi += i - 1
                next
              end
              return fail.()
            when "A", "a"
              idx = nil
              DAY_NAMES.each_with_index do |name, k|
                if head_match?(str, si, name, name.size)
                  si += name.size
                  idx = k
                elsif head_match?(str, si, name, 3)
                  si += 3
                  idx = k
                end
                break if idx
              end
              return fail.() unless idx
              h[:wday] = idx
            when "B", "b", "h"
              idx = nil
              MONTH_NAMES.each_with_index do |name, k|
                if head_match?(str, si, name, name.size)
                  si += name.size
                  idx = k
                elsif head_match?(str, si, name, 3)
                  si += 3
                  idx = k
                end
                break if idx
              end
              return fail.() unless idx
              h[:mon] = idx + 1
            when "C"
              n, l = read_digits(str, si, num_pattern?(fmt, fi + 1) ? 2 : Float::INFINITY) || (return fail.())
              si += l
              h[:_cent] = n
            when "c" then si = strptime_internal(str, si, "%a %b %e %H:%M:%S %Y", h, enc) or return nil
            when "D", "x" then si = strptime_internal(str, si, "%m/%d/%y", h, enc) or return nil
            when "d", "e"
              if str.getbyte(si) == 32
                si += 1
                n, l = read_digits(str, si, 1) || (return fail.())
              else
                n, l = read_digits(str, si, 2) || (return fail.())
              end
              si += l
              return fail.() unless n >= 1 && n <= 31
              h[:mday] = n
            when "F" then si = strptime_internal(str, si, "%Y-%m-%d", h, enc) or return nil
            when "G"
              n, l = read_digits(str, si, num_pattern?(fmt, fi + 1) ? 4 : Float::INFINITY) || (return fail.())
              si += l
              h[:cwyear] = n
            when "g"
              n, l = read_digits(str, si, 2) || (return fail.())
              si += l
              return fail.() unless n >= 0 && n <= 99
              h[:cwyear] = n
              h[:_cent] = n >= 69 ? 19 : 20 if h[:_cent].nil?
            when "H", "k", "I", "l"
              if str.getbyte(si) == 32
                si += 1
                n, l = read_digits(str, si, 1) || (return fail.())
              else
                n, l = read_digits(str, si, 2) || (return fail.())
              end
              si += l
              if ch == "H" || ch == "k"
                return fail.() unless n >= 0 && n <= 24
              else
                return fail.() unless n >= 1 && n <= 12
              end
              h[:hour] = n
            when "j"
              n, l = read_digits(str, si, 3) || (return fail.())
              si += l
              return fail.() unless n >= 1 && n <= 366
              h[:yday] = n
            when "L", "N"
              sign = 1
              if sign?(str.getbyte(si))
                sign = -1 if str.getbyte(si) == 45
                si += 1
              end
              osi = si
              n, l = read_digits(str, si, num_pattern?(fmt, fi + 1) ? (ch == "L" ? 3 : 9) : Float::INFINITY) || (return fail.())
              si += l
              n = -n if sign == -1
              h[:sec_fraction] = Rational(n, 10**(si - osi))
            when "M"
              n, l = read_digits(str, si, 2) || (return fail.())
              si += l
              return fail.() unless n >= 0 && n <= 59
              h[:min] = n
            when "m"
              n, l = read_digits(str, si, 2) || (return fail.())
              si += l
              return fail.() unless n >= 1 && n <= 12
              h[:mon] = n
            when "n", "t" then si = strptime_internal(str, si, " ", h, enc) or return nil
            when "P", "p"
              return fail.() if slen - si < 2
              c0 = str.getbyte(si)
              hour = (c0 == 80 || c0 == 112) ? 12 : 0
              return fail.() if hour == 0 && !(c0 == 65 || c0 == 97)
              c1 = str.getbyte(si + 1)
              if c1 == 46
                return fail.() if slen - si < 4 || str.getbyte(si + 3) != 46
                si += 2
                c1 = str.getbyte(si)
              end
              return fail.() unless c1 == 77 || c1 == 109
              si += 2
              h[:_merid] = hour
            when "Q", "s"
              sign = 1
              if str.getbyte(si) == 45
                sign = -1
                si += 1
              end
              n, l = read_digits(str, si, Float::INFINITY) || (return fail.())
              si += l
              n = -n if sign == -1
              h[:seconds] = ch == "Q" ? Rational(n, 1000) : n
            when "R" then si = strptime_internal(str, si, "%H:%M", h, enc) or return nil
            when "r" then si = strptime_internal(str, si, "%I:%M:%S %p", h, enc) or return nil
            when "S"
              n, l = read_digits(str, si, 2) || (return fail.())
              si += l
              return fail.() unless n >= 0 && n <= 60
              h[:sec] = n
            when "T", "X" then si = strptime_internal(str, si, "%H:%M:%S", h, enc) or return nil
            when "U", "W"
              n, l = read_digits(str, si, 2) || (return fail.())
              si += l
              return fail.() unless n >= 0 && n <= 53
              h[ch == "U" ? :wnum0 : :wnum1] = n
            when "u"
              n, l = read_digits(str, si, 1) || (return fail.())
              si += l
              return fail.() unless n >= 1 && n <= 7
              h[:cwday] = n
            when "V"
              n, l = read_digits(str, si, 2) || (return fail.())
              si += l
              return fail.() unless n >= 1 && n <= 53
              h[:cweek] = n
            when "v" then si = strptime_internal(str, si, "%e-%b-%Y", h, enc) or return nil
            when "w"
              n, l = read_digits(str, si, 1) || (return fail.())
              si += l
              return fail.() unless n >= 0 && n <= 6
              h[:wday] = n
            when "Y"
              sign = 1
              if sign?(str.getbyte(si))
                sign = -1 if str.getbyte(si) == 45
                si += 1
              end
              n, l = read_digits(str, si, num_pattern?(fmt, fi + 1) ? 4 : Float::INFINITY) || (return fail.())
              si += l
              n = -n if sign == -1
              h[:year] = n
            when "y"
              n, l = read_digits(str, si, 2) || (return fail.())
              si += l
              return fail.() unless n >= 0 && n <= 99
              h[:year] = n
              h[:_cent] = n >= 69 ? 19 : 20 if h[:_cent].nil?
            when "Z", "z"
              rest = str.byteslice(si, slen - si).force_encoding(enc)
              m = STRPTIME_ZONE.match(rest) or return fail.()
              s = m[1]
              si += s.bytesize
              h[:zone] = s
              h[:offset] = zone_to_diff(s)
            when "%"
              return fail.() if str.getbyte(si) != 37
              si += 1
            when "+" then si = strptime_internal(str, si, "%a %b %e %H:%M:%S %Z %Y", h, enc) or return nil
            else
              return fail.() if str.getbyte(si) != 37
              si += 1
              if fi < flen
                return fail.() if si >= slen || str.getbyte(si) != fmt.getbyte(fi)
                si += 1
              end
            end
            break
          end
          unless ordinal
            fi += 1
            next
          end
        end
        return fail.() if str.getbyte(si) != fmt.getbyte(fi)
        si += 1
        fi += 1
      end
      si
    end

    def strptime(str, fmt, enc)
      h = {}
      si = strptime_internal(str.b, 0, fmt.b, h, enc)
      return nil if si.nil? || h[:_fail]
      sl = str.bytesize
      h[:leftover] = str.byteslice(si, sl - si).force_encoding(enc) if sl > si
      if (cent = h.delete(:_cent))
        h[:cwyear] = h[:cwyear] + cent * 100 unless h[:cwyear].nil?
        h[:year] = h[:year] + cent * 100 unless h[:year].nil?
      end
      if (merid = h.delete(:_merid))
        h[:hour] = h[:hour] % 12 + merid unless h[:hour].nil?
      end
      h
    end
  end
  private_constant :Parse

  class << self
    def __zone_to_diff(s) = Parse.zone_to_diff(s) # :nodoc:

    def _parse(str, comp = true, limit: 128)
      str = __check_limit(str, limit)
      raise ArgumentError, "string should have ASCII compatible encoding" unless str.encoding.ascii_compatible?
      Parse.parse(str, comp)
    end

    def _iso8601(str, limit: 128) = __frags(:iso8601, str, limit)
    def _rfc3339(str, limit: 128) = __frags(:rfc3339, str, limit)
    def _xmlschema(str, limit: 128) = __frags(:xmlschema, str, limit)
    def _rfc2822(str, limit: 128) = __frags(:rfc2822, str, limit)
    alias _rfc822 _rfc2822
    def _httpdate(str, limit: 128) = __frags(:httpdate, str, limit)
    def _jisx0301(str, limit: 128) = __frags(:jisx0301, str, limit)

    def _strptime(str, fmt = (nofmt = true; "%F"))
      __strptime_hash(str, fmt, nofmt)
    end

    def parse(str = "-4712-01-01", comp = true, start = ITALY, **opt)
      __new_by_frags(_parse(str, comp, **opt), start)
    end

    def strptime(str = "-4712-01-01", fmt = "%F", start = ITALY)
      __new_by_frags(_strptime(str, fmt), start)
    end

    def iso8601(str = "-4712-01-01", start = ITALY, **opt) = __new_by_frags(_iso8601(str, **opt), start)
    def rfc3339(str = "-4712-01-01T00:00:00+00:00", start = ITALY, **opt) = __new_by_frags(_rfc3339(str, **opt), start)
    def xmlschema(str = "-4712-01-01", start = ITALY, **opt) = __new_by_frags(_xmlschema(str, **opt), start)
    def rfc2822(str = "Mon, 1 Jan -4712 00:00:00 +0000", start = ITALY, **opt) = __new_by_frags(_rfc2822(str, **opt), start)
    alias rfc822 rfc2822
    def httpdate(str = "Mon, 01 Jan -4712 00:00:00 GMT", start = ITALY, **opt) = __new_by_frags(_httpdate(str, **opt), start)
    def jisx0301(str = "-4712-01-01", start = ITALY, **opt) = __new_by_frags(_jisx0301(str, **opt), start)

    private

    def __check_limit(str, limit)
      str = String.try_convert(str) || raise(TypeError, "no implicit conversion of #{str.nil? ? 'nil' : str.class} into String")
      return str if limit.nil?
      raise TypeError, "no implicit conversion of #{limit.class} into Integer" unless limit.is_a?(Numeric)
      lim = limit.to_int
      if lim >= 0 && str.bytesize > lim
        raise ArgumentError, "string length (#{str.bytesize}) exceeds the limit #{lim}"
      end
      str
    end

    def __frags(kind, str, limit)
      str = __check_limit(str, limit) unless str.nil?
      return {} if str.nil?
      Parse.__send__(kind, str)
    end

    def __strptime_hash(str, fmt, nofmt)
      str = String.try_convert(str) || raise(TypeError, "no implicit conversion of #{str.nil? ? 'nil' : str.class} into String")
      unless nofmt
        fmt = String.try_convert(fmt) || raise(TypeError, "no implicit conversion of #{fmt.nil? ? 'nil' : fmt.class} into String")
      end
      raise ArgumentError, "string should have ASCII compatible encoding" unless str.encoding.ascii_compatible?
      raise ArgumentError, "format should have ASCII compatible encoding" unless nofmt || fmt.encoding.ascii_compatible?
      Parse.strptime(str, fmt, str.encoding)
    end

    def __rewrite_frags(h)
      seconds = h.delete(:seconds)
      unless seconds.nil?
        offset = h[:offset]
        seconds += offset unless offset.nil?
        d = seconds.div(86400)
        fr = seconds % 86400
        hr = fr.div(3600)
        fr %= 3600
        mi = fr.div(60)
        fr %= 60
        s = fr.div(1)
        fr %= 1
        h[:jd] = 2440588 + d
        h[:hour] = hr
        h[:min] = mi
        h[:sec] = s
        h[:sec_fraction] = fr
      end
      h
    end

    COMPLETE_TAB = [
      [:time, %i[hour min sec]],
      [nil, %i[jd]],
      [:ordinal, %i[year yday hour min sec]],
      [:civil, %i[year mon mday hour min sec]],
      [:commercial, %i[cwyear cweek cwday hour min sec]],
      [:wday, %i[wday hour min sec]],
      [:wnum0, %i[year wnum0 wday hour min sec]],
      [:wnum1, %i[year wnum1 wday hour min sec]],
      [nil, %i[cwyear cweek wday hour min sec]],
      [nil, %i[year wnum0 cwday hour min sec]],
      [nil, %i[year wnum1 cwday hour min sec]],
    ].freeze
    private_constant :COMPLETE_TAB

    def __today_field(d, e)
      case e
      when :wnum0 then d.__send__(:__wnum0)
      when :wnum1 then d.__send__(:__wnum1)
      when :hour, :min, :sec then 0
      else d.__send__(e)
      end
    end

    def __complete_frags(klass, h)
      k = a = nil
      eno = 0
      COMPLETE_TAB.each do |x|
        n = x[1].count { |e| !h[e].nil? }
        if n > eno
          eno = n
          k, a = x
        end
      end
      d = nil
      if !k.nil? && a.size > eno
        case k
        when :ordinal
          if h[:year].nil?
            d ||= Date.today
            h[:year] = d.year
          end
          h[:yday] = 1 if h[:yday].nil?
        when :civil, :commercial, :wnum0, :wnum1
          a.each do |e|
            break unless h[e].nil?
            d ||= Date.today
            h[e] = __today_field(d, e)
          end
          case k
          when :civil
            h[:mon] = 1 if h[:mon].nil?
            h[:mday] = 1 if h[:mday].nil?
          when :commercial
            h[:cweek] = 1 if h[:cweek].nil?
            h[:cwday] = 1 if h[:cwday].nil?
          when :wnum0
            h[:wnum0] = 0 if h[:wnum0].nil?
            h[:wday] = 0 if h[:wday].nil?
          when :wnum1
            h[:wnum1] = 0 if h[:wnum1].nil?
            h[:wday] = 1 if h[:wday].nil?
          end
        when :wday
          d ||= Date.today
          h[:jd] = (d - d.wday + h[:wday]).jd
        end
      end
      if k == :time && klass <= DateTime
        d ||= Date.today
        h[:jd] = d.jd if h[:jd].nil?
      end
      h[:hour] = 0 if h[:hour].nil?
      h[:min] = 0 if h[:min].nil?
      if h[:sec].nil?
        h[:sec] = 0
      elsif h[:sec] > 59
        h[:sec] = 59
      end
      h
    end

    def __valid_date_frags(h, sg)
      jd = h[:jd]
      return jd unless jd.nil?
      if !(yd = h[:yday]).nil? && !(y = h[:year]).nil?
        jd = Calc.valid_ordinal(y, Calc.num2int(yd), sg) and return jd
      end
      if !(md = h[:mday]).nil? && !(mo = h[:mon]).nil? && !(y = h[:year]).nil?
        jd = Calc.valid_civil(y, Calc.num2int(mo), Calc.num2int(md), sg) and return jd
      end
      wd = h[:cwday]
      if wd.nil?
        wd = h[:wday]
        wd = 7 if !wd.nil? && wd == 0
      end
      if !wd.nil? && !(w = h[:cweek]).nil? && !(y = h[:cwyear]).nil?
        jd = Calc.valid_commercial(y, Calc.num2int(w), Calc.num2int(wd), sg) and return jd
      end
      wd = h[:wday]
      if wd.nil?
        wd = h[:cwday]
        wd = 0 if !wd.nil? && wd == 7
      end
      if !wd.nil? && !(w = h[:wnum0]).nil? && !(y = h[:year]).nil?
        jd = Calc.valid_weeknum(y, Calc.num2int(w), Calc.num2int(wd), 0, sg) and return jd
      end
      wd = h[:wday]
      wd = h[:cwday] if wd.nil?
      wd = (wd - 1) % 7 unless wd.nil?
      if !wd.nil? && !(w = h[:wnum1]).nil? && !(y = h[:year]).nil?
        jd = Calc.valid_weeknum(y, Calc.num2int(w), Calc.num2int(wd), 1, sg) and return jd
      end
      nil
    end

    def __frags_sg(sg)
      unless Calc.valid_start?(Calc.num2dbl(sg))
        warn("invalid start is ignored", uplevel: 3) if $VERBOSE
        sg = ITALY
      end
      Calc.num2dbl(sg)
    end

    def __new_by_frags(h, sg)
      sg = __frags_sg(sg)
      raise Error, "invalid date" if h.nil?
      if h[:jd].nil? && h[:yday].nil? && !h[:year].nil? && !h[:mon].nil? && !h[:mday].nil?
        jd = Calc.valid_civil(h[:year], Calc.num2int(h[:mon]), Calc.num2int(h[:mday]), sg)
      else
        h = __complete_frags(self, __rewrite_frags(h))
        jd = __valid_date_frags(h, sg)
      end
      raise Error, "invalid date" if jd.nil?
      __new_raw(jd, 0, 0, 0, sg, false)
    end
  end
end

class Date
  class << DateTime
    def _strptime(str, fmt = (nofmt = true; "%FT%T%z"))
      __strptime_hash(str, fmt, nofmt)
    end

    def parse(str = "-4712-01-01T00:00:00+00:00", comp = true, start = ITALY, **opt)
      __new_by_frags(_parse(str, comp, **opt), start)
    end

    def strptime(str = "-4712-01-01T00:00:00+00:00", fmt = "%FT%T%z", start = ITALY)
      __new_by_frags(_strptime(str, fmt), start)
    end

    def iso8601(str = "-4712-01-01T00:00:00+00:00", start = ITALY, **opt) = __new_by_frags(_iso8601(str, **opt), start)
    def rfc3339(str = "-4712-01-01T00:00:00+00:00", start = ITALY, **opt) = __new_by_frags(_rfc3339(str, **opt), start)
    def xmlschema(str = "-4712-01-01T00:00:00+00:00", start = ITALY, **opt) = __new_by_frags(_xmlschema(str, **opt), start)
    def rfc2822(str = "Mon, 1 Jan -4712 00:00:00 +0000", start = ITALY, **opt) = __new_by_frags(_rfc2822(str, **opt), start)
    alias rfc822 rfc2822
    def httpdate(str = "Mon, 01 Jan -4712 00:00:00 GMT", start = ITALY, **opt) = __new_by_frags(_httpdate(str, **opt), start)
    def jisx0301(str = "-4712-01-01T00:00:00+00:00", start = ITALY, **opt) = __new_by_frags(_jisx0301(str, **opt), start)

    private

    def __new_by_frags(h, sg)
      sg = __frags_sg(sg)
      raise Error, "invalid date" if h.nil?
      if h[:jd].nil? && h[:yday].nil? && !h[:year].nil? && !h[:mon].nil? && !h[:mday].nil?
        jd = Calc.valid_civil(h[:year], Calc.num2int(h[:mon]), Calc.num2int(h[:mday]), sg)
        h[:hour] = 0 if h[:hour].nil?
        h[:min] = 0 if h[:min].nil?
        if h[:sec].nil?
          h[:sec] = 0
        elsif h[:sec] == 60
          h[:sec] = 59
        end
      else
        h = __complete_frags(self, __rewrite_frags(h))
        jd = __valid_date_frags(h, sg)
      end
      raise Error, "invalid date" if jd.nil?
      tm = Calc.valid_time(Calc.num2int(h[:hour]), Calc.num2int(h[:min]), Calc.num2int(h[:sec])) or raise Error, "invalid date"
      hr, mi, s = tm
      df = hr * 3600 + mi * 60 + s
      t = h[:sec_fraction]
      sf = t.nil? ? 0 : t * Calc::NS
      t = h[:offset]
      if t.nil?
        of = 0
      else
        of = Calc.num2int(t)
        if of < -Calc::DAY || of > Calc::DAY
          of = 0
          warn("invalid offset is ignored", uplevel: 2) if $VERBOSE
        end
      end
      ujd, udf = __local_to_utc(jd, df, of)
      __new_raw(ujd, udf, sf, of, sg, true)
    end
  end
end

class Date
  # lib/date.rb
  class Infinity < Numeric # :nodoc:
    def initialize(d = 1) = @d = d <=> 0
    def d = @d
    protected :d
    def zero? = false
    def finite? = false
    def infinite? = d.nonzero?
    def nan? = d.zero?
    def abs = self.class.new
    def -@ = self.class.new(-d)
    def +@ = self.class.new(+d)

    def <=>(other)
      case other
      when Infinity then return d <=> other.d
      when Float::INFINITY then return d <=> 1
      when -Float::INFINITY then return d <=> -1
      when Numeric then return d
      else
        begin
          l, r = other.coerce(self)
          return l <=> r
        rescue NoMethodError
        end
      end
      nil
    end

    def coerce(other)
      case other
      when Numeric then [-d, d]
      else super
      end
    end

    def to_f
      return 0 if @d == 0
      @d > 0 ? Float::INFINITY : -Float::INFINITY
    end
  end
end
